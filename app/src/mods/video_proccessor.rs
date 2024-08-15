use crate::api::do_anime_task::handle_error;
use anyhow::Error;
use ffmpeg::{
    codec, decoder, encoder, format, frame, media, picture, software, subtitle::Rect, Dictionary,
    Packet, Rational,
};
use ffmpeg_next as ffmpeg;
use ffmpeg_next::ffi::{av_hwdevice_get_type_name, av_hwdevice_iterate_types, AVHWDeviceType};
use regex::Regex;
use rsubs_lib::{ssa, vtt};
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::CStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::time::Instant;

#[derive(Debug)]
pub struct SubtitleInfo {
    pub index: usize,
    pub title: String,
    pub format: String,
}

#[allow(dead_code)]
pub async fn get_subtitle_info(path: &String) -> Result<Vec<SubtitleInfo>, Error> {
    ffmpeg::init().unwrap();

    let ictx = format::input(path).map_err(|e| handle_error(e, "Failed to get subtitle info"))?;
    let mut subtitle_vec: Vec<SubtitleInfo> = vec![];
    let extension = path.split(".").last().unwrap();

    for (ist_index, ist) in ictx.streams().enumerate() {
        if ist.parameters().medium() == media::Type::Subtitle {
            let title = match extension {
                "mp4" => ist.metadata().get("handler_name").unwrap_or("").to_string(),
                "mkv" => ist.metadata().get("title").unwrap_or("").to_string(),
                _ => String::new(),
            };
            subtitle_vec.push(SubtitleInfo {
                index: ist_index,
                title,
                format: codec::context::Context::from_parameters(ist.parameters())?
                    .id()
                    .name()
                    .to_string(),
            });
        }
    }
    Ok(subtitle_vec)
}

#[allow(dead_code)]
pub async fn extract_subtitle(path: &String) -> Result<Vec<String>, Error> {
    let subtitle_vec = get_subtitle_info(&path)
        .await
        .map_err(|e| handle_error(e, "Failed to get subtitle info"))?;

    let mut output_subtitle_file: Vec<String> = vec![];

    if subtitle_vec.len() == 0 {
        return Err(Error::msg("Failed to get any subtitle stream"));
    }

    let video_name = path.split(".").next().unwrap();
    let extension = path.split(".").last().unwrap();

    for subtitle in subtitle_vec {
        let subtitle_extension = match subtitle.format.as_str() {
            "ass" => "ass",
            "srt" => "srt",
            _ => "ass",
        };

        let output_file = format!("{} - {}.{}", video_name, subtitle.title, subtitle_extension);

        let ret: Result<(), Error> = match extension {
            "mp4" => extract_mp4_subtitle(subtitle.index, &path, &output_file).await,
            "mkv" => extract_mkv_subtitle(subtitle.index, &path, &output_file).await,
            _ => Err(Error::msg("Unsupported file extension")),
        };

        if ret.is_ok() {
            let vtt_path = output_file.split(".").next().unwrap().to_string() + ".vtt";
            if let Ok(_) = trans_subtitle_to_vtt(&output_file, &vtt_path).await {
                let subtitle_name = vtt_path.split("/").last().unwrap().to_string();
                output_subtitle_file.push(subtitle_name);
                log::info!("Successfully extracted subtitle from {}", path);
            } else {
                log::warn!("Failed to trans format to vtt for [{}]", path);
                continue;
            }
            if let Err(e) = fs::remove_file(&output_file) {
                log::warn!("Failed to remove tmp file [{}], {}", output_file, e);
                continue;
            }

            if let Err(e) = strip_srt_tags_from_vtt(&vtt_path).await {
                log::warn!("Failed to remove srt tag for [{}], {}", vtt_path, e);
                continue;
            }
        } else {
            log::warn!("Failed to extract subtitle from {}", path);
            continue;
        }
    }

    Ok(output_subtitle_file)
}

#[allow(dead_code)]
async fn extract_mkv_subtitle(
    subtitle_stream_index: usize,
    input_file: &String,
    output_file: &String,
) -> Result<(), Error> {
    ffmpeg::init().unwrap();

    let mut ictx = format::input(&input_file).unwrap();
    let mut octx = format::output(&output_file).unwrap();

    let ist = ictx.stream(subtitle_stream_index).unwrap();

    let ist_time_base = ist.time_base();
    let mut ost = octx.add_stream(encoder::find(codec::Id::None)).unwrap();
    ost.set_parameters(ist.parameters());
    unsafe {
        (*ost.parameters().as_mut_ptr()).codec_tag = 0;
    }

    octx.set_metadata(ictx.metadata().to_owned());
    match octx.write_header() {
        Ok(_) => {
            for (stream, mut packet) in ictx.packets() {
                if stream.index() == subtitle_stream_index {
                    let ost = octx.stream(0).unwrap();
                    packet.rescale_ts(ist_time_base, ost.time_base());
                    packet.set_stream(0);
                    packet.write_interleaved(&mut octx).unwrap();
                }
            }
            octx.write_trailer().unwrap();
            Ok(())
        }
        Err(e) => {
            log::warn!("failed to write header for [{}], {}", input_file, e);
            Err(Error::from(e))
        }
    }
}

#[allow(dead_code)]
pub async fn trans_subtitle_to_vtt(
    intput_file: &String,
    output_file: &String,
) -> Result<(), Error> {
    vtt::VTTFile::from(ssa::parse(intput_file.to_string()).unwrap())
        .to_file(output_file)
        .unwrap();
    Ok(())
}

#[allow(dead_code)]
pub async fn extract_mp4_subtitle(
    subtitle_stream_index: usize,
    input_file: &String,
    output_file: &String,
) -> Result<(), Error> {
    ffmpeg::init().unwrap();

    let mut ictx = format::input(&input_file).unwrap();
    let ist = ictx.stream(subtitle_stream_index).unwrap();

    let context_decoder = ffmpeg::codec::context::Context::from_parameters(ist.parameters())?;
    let mut decoder = context_decoder.decoder().subtitle()?;

    let mut file = fs::File::create(output_file)?;
    let config = load_config("./config/subtitle_config.yaml").unwrap();
    add_basic_subtitle_info(&file, &config)?;

    for (stream, packet) in ictx.packets() {
        if stream.index() == subtitle_stream_index {
            let mut out: ffmpeg::Subtitle = Default::default();
            decoder.decode(&packet, &mut out).unwrap();

            for rect in out.rects() {
                match rect {
                    Rect::Ass(rect_ass) => {
                        let start_time =
                            format_timestamp(packet.pts().unwrap(), stream.time_base())?;
                        let end_time = format_timestamp(
                            packet.pts().unwrap() + packet.duration(),
                            stream.time_base(),
                        )?;

                        let subtitle_ctx = format!(
                            "Dialogue: 0,{},{},Default,,0,0,0,,{}",
                            start_time,
                            end_time,
                            rect_ass.get().split(",,").last().unwrap()
                        );
                        writeln!(file, "{}", subtitle_ctx).unwrap();
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

fn format_timestamp(timestamp: i64, time_base: Rational) -> Result<String, Error> {
    let hours: i32;
    let mut mins: i32;
    let mut secs: i32;
    let us: i32;

    let duration = timestamp * ((f64::from(time_base) * 1000000.0) as i64);

    secs = (duration / 1000000) as i32;
    us = (duration % 1000000) as i32;
    mins = secs / 60;
    secs %= 60;
    hours = mins / 60;
    mins %= 60;

    let buffer = format!(
        "{0}:{1:02}:{2:02}.{3:02}",
        hours,
        mins,
        secs,
        (100 * us) / 1000000
    );
    Ok(buffer)
}

#[derive(Deserialize)]
struct SubtitleConfig {
    script_info: ScriptInfo,
    fonts: Fonts,
    styles: Styles,
    events: Events,
}

#[derive(Deserialize)]
struct ScriptInfo {
    title: String,
    script_type: String,
    wrap_style: i32,
    scaled_border_and_shadow: String,
    play_res_x: i32,
    play_res_y: i32,
}

#[derive(Deserialize)]
struct Fonts {
    subsets: Vec<String>,
}

#[derive(Deserialize)]
struct Styles {
    formats: String,
    entries: Vec<String>,
}

#[derive(Deserialize)]
struct Events {
    format: String,
}

fn load_config(file_path: &str) -> Result<SubtitleConfig, Error> {
    let config_content = fs::read_to_string(file_path).unwrap();
    let config: SubtitleConfig = serde_yml::from_str(&config_content).unwrap();
    Ok(config)
}

fn add_basic_subtitle_info(mut file: &fs::File, config: &SubtitleConfig) -> Result<(), Error> {
    writeln!(file, "[Script Info]")?;
    for subset in &config.fonts.subsets {
        writeln!(file, "; Font Subset: {}", subset)?;
    }
    writeln!(file, "Title: {}", config.script_info.title)?;
    writeln!(file, "ScriptType: {}", config.script_info.script_type)?;
    writeln!(file, "WrapStyle: {}", config.script_info.wrap_style)?;
    writeln!(file, "ScaledBorderAndShadow: {}", config.script_info.scaled_border_and_shadow)?;
    writeln!(file, "PlayResX: {}", config.script_info.play_res_x)?;
    writeln!(file, "PlayResY: {}", config.script_info.play_res_y)?;
    writeln!(file, "")?;

    writeln!(file, "[V4+ Styles]")?;
    writeln!(file, "Format: {}", config.styles.formats)?;
    for entry in &config.styles.entries {
        writeln!(file, "Style: {}", entry)?;
    }
    writeln!(file, "")?;

    writeln!(file, "[Events]")?;
    writeln!(file, "Format: {}", config.events.format)?;
    Ok(())
}

#[allow(dead_code)]
pub async fn strip_srt_tags_from_vtt(path: &String) -> Result<(), Error> {
    let lines: Vec<String> = read_lines(&path)?.filter_map(Result::ok).collect();

    let html_tag_pattern = Regex::new(r"<.*?>").unwrap();
    let srt_style_pattern = Regex::new(r"\{.*?\}").unwrap();
    let jp_pattern = Regex::new(r"<.*?(JP|JPN|STAFF).*?>").unwrap();

    let mut skip_indices = vec![];

    for (i, line) in lines.iter().enumerate() {
        if jp_pattern.is_match(&line) {
            let start = if i >= 3 { i - 3 } else { 0 };
            skip_indices.extend(start..=i);
        }
    }

    let mut cleaned_lines: Vec<String> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if !skip_indices.contains(&i) {
            let without_html_tags = html_tag_pattern.replace_all(&line, "").to_string();
            let cleaned_line = srt_style_pattern
                .replace_all(&without_html_tags, "")
                .to_string();
            cleaned_lines.push(cleaned_line);
        }
    }

    let mut timestamp_count = 1;
    let mut output_lines: Vec<String> = Vec::new();

    for line in cleaned_lines.iter() {
        if line.starts_with("00:") {
            output_lines.pop();
            output_lines.push(format!("{}", timestamp_count));
            timestamp_count += 1;
        }
        output_lines.push(line.to_string());
    }

    let mut output_file = OpenOptions::new().write(true).truncate(true).open(path)?;

    for line in output_lines {
        writeln!(output_file, "{}", line)?;
    }
    Ok(())
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

#[allow(dead_code)]
pub fn get_av_hwaccels() -> Result<Vec<String>, Error> {
    let mut device_type = AVHWDeviceType::AV_HWDEVICE_TYPE_NONE;
    let mut av_codec_vec = Vec::new();

    unsafe {
        while {
            device_type = av_hwdevice_iterate_types(device_type);
            device_type != AVHWDeviceType::AV_HWDEVICE_TYPE_NONE
        } {
            let type_name = av_hwdevice_get_type_name(device_type);

            if !type_name.is_null() {
                let c_str = CStr::from_ptr(type_name);
                let str_slice = c_str.to_str()?;
                av_codec_vec.push(str_slice.to_string());
            }
        }
    }

    Ok(av_codec_vec)
}

/*
    videotoolbox    -> h264_videotoolbox    -> Macos, M1/2/3
    cuda            -> h264_nvenc           -> Windows, Nvidia GPU,
    vaapi           -> h264_vaapi           -> Linux, Intel GPU, AMD GPU
    qsv             -> h264_qsv             -> Windows, Linux, Intel GPU,
    dxva2                                   -> Windows, Intel GPU, AMD GPU, NVIDIA GPU,
    d3d11va                                 -> Windows, Intel GPU, AMD GPU, NVIDIA GPU, DX11
    d3d12va                                 -> Windows, Intel GPU, AMD GPU, NVIDIA GPU, DX12
    opencl
    vulkan
*/

#[allow(dead_code)]
fn trans_hwaccels_2_codec_name(av_codec_vec: Vec<String>) -> String {
    let os_name = std::env::consts::OS;

    let codec_name = match os_name {
        "macos" => {
            if av_codec_vec.contains(&"videotoolbox".to_string()) {
                "h264_videotoolbox"
            } else {
                "h264"
            }
        }
        "windows" | "linux" => {
            if av_codec_vec.contains(&"cuda".to_string()) {
                "h264_nvenc"
            } else if av_codec_vec.contains(&"qsv".to_string()) {
                "h264_qsv"
            } else if av_codec_vec.contains(&"vaapi".to_string()) {
                "h264_vaapi"
            } else {
                "h264"
            }
        }
        _ => "h264",
    };

    codec_name.to_string()
}

struct Transcoder {
    ost_index: usize,
    decoder: decoder::Video,
    input_time_base: Rational,
    encoder: encoder::Video,
    logging_enabled: bool,
    frame_count: usize,
    last_log_frame_count: usize,
    starting_time: Instant,
    last_log_time: Instant,
}

impl Transcoder {
    fn new(
        ist: &format::stream::Stream,
        octx: &mut format::context::Output,
        ost_index: usize,
        x264_opts: Dictionary,
        enable_logging: bool,
    ) -> Result<Self, ffmpeg::Error> {
        let global_header = octx.format().flags().contains(format::Flags::GLOBAL_HEADER);
        let decoder = ffmpeg::codec::context::Context::from_parameters(ist.parameters())?
            .decoder()
            .video()?;

        let codec = match get_av_hwaccels() {
            Ok(av_codec_vec) if !av_codec_vec.is_empty() => {
                let codec_name = trans_hwaccels_2_codec_name(av_codec_vec);
                log::info!("Use hardware decoder [{}] to speed up decoding", codec_name);
                encoder::find_by_name(&codec_name)
            }
            _ => {
                log::info!("Does not support hardware accelerated decoding, uses CPU decoding");
                encoder::find(codec::Id::H264)
            }
        };

        let mut ost = octx.add_stream(codec)?;

        let mut encoder =
            codec::context::Context::new_with_codec(codec.ok_or(ffmpeg::Error::InvalidData)?)
                .encoder()
                .video()?;
        ost.set_parameters(&encoder);
        encoder.set_height(decoder.height());
        encoder.set_width(decoder.width());
        encoder.set_aspect_ratio(decoder.aspect_ratio());
        encoder.set_format(format::Pixel::NV12);
        encoder.set_frame_rate(decoder.frame_rate());
        encoder.set_time_base(ist.time_base());

        if global_header {
            encoder.set_flags(codec::Flags::GLOBAL_HEADER);
        }

        let opened_encoder = encoder
            .open_with(x264_opts)
            .expect("error opening x264 with supplied settings");
        ost.set_parameters(&opened_encoder);

        Ok(Self {
            ost_index,
            decoder,
            input_time_base: ist.time_base(),
            encoder: opened_encoder,
            logging_enabled: enable_logging,
            frame_count: 0,
            last_log_frame_count: 0,
            starting_time: Instant::now(),
            last_log_time: Instant::now(),
        })
    }

    fn send_packet_to_decoder(&mut self, packet: &Packet) {
        self.decoder.send_packet(packet).unwrap();
    }

    fn send_eof_to_decoder(&mut self) {
        self.decoder.send_eof().unwrap();
    }

    fn receive_and_process_decoded_frames(
        &mut self,
        octx: &mut format::context::Output,
        ost_time_base: Rational,
    ) {
        let mut frame = frame::Video::empty();
        while self.decoder.receive_frame(&mut frame).is_ok() {
            self.frame_count += 1;
            let timestamp = frame.timestamp();

            self.log_progress(f64::from(
                Rational(timestamp.unwrap_or(0) as i32, 1) * self.input_time_base,
            ));

            if frame.format() != format::Pixel::NV12 {
                let mut converted_frame = frame::Video::empty();
                converted_frame.set_format(format::Pixel::NV12);
                converted_frame.set_width(frame.width());
                converted_frame.set_height(frame.height());

                let mut converter = software::scaling::context::Context::get(
                    frame.format(),
                    frame.width(),
                    frame.height(),
                    format::Pixel::NV12,
                    frame.width(),
                    frame.height(),
                    software::scaling::Flags::BILINEAR,
                )
                .unwrap();

                if let Err(e) = converter.run(&frame, &mut converted_frame) {
                    log::error!("Error during frame conversion: {:?}", e);
                    return;
                }
                frame = converted_frame;
            }
            frame.set_pts(timestamp);
            frame.set_kind(picture::Type::None);
            self.send_frame_to_encoder(&frame);
            self.receive_and_process_encoded_packets(octx, ost_time_base);
        }
    }

    fn send_frame_to_encoder(&mut self, frame: &frame::Video) {
        self.encoder.send_frame(frame).unwrap();
    }

    fn send_eof_to_encoder(&mut self) {
        self.encoder.send_eof().unwrap();
    }

    fn receive_and_process_encoded_packets(
        &mut self,
        octx: &mut format::context::Output,
        ost_time_base: Rational,
    ) {
        let mut encoded = Packet::empty();
        while self.encoder.receive_packet(&mut encoded).is_ok() {
            encoded.set_stream(self.ost_index);
            encoded.rescale_ts(self.input_time_base, ost_time_base);
            encoded.write_interleaved(octx).unwrap();
        }
    }

    fn log_progress(&mut self, timestamp: f64) {
        if !self.logging_enabled
            || (self.frame_count - self.last_log_frame_count < 100
                && self.last_log_time.elapsed().as_secs_f64() < 1.0)
        {
            return;
        }
        log::info!(
            "time elpased: \t{:8.2}\tframe count: {:8}\ttimestamp: {:8.2}",
            self.starting_time.elapsed().as_secs_f64(),
            self.frame_count,
            timestamp
        );

        self.last_log_frame_count = self.frame_count;
        self.last_log_time = Instant::now();
    }
}

#[allow(dead_code)]
pub async fn trans_mkv_2_mp4(input_file: &String) -> Result<(), Error> {
    ffmpeg::init().unwrap();
    let output_file = format!("{}.mp4", input_file.split(".").next().unwrap());

    let mut x264_opts = Dictionary::new();
    x264_opts.set("preset", "medium");

    let mut ictx = format::input(&input_file).unwrap();
    let mut octx = format::output(&output_file).unwrap();

    format::context::input::dump(&ictx, 0, Some(&input_file));

    let best_video_stream_index = ictx
        .streams()
        .best(media::Type::Video)
        .map(|stream| stream.index());
    let mut stream_mapping: Vec<isize> = vec![0; ictx.nb_streams() as _];
    let mut ist_time_bases = vec![Rational(0, 0); ictx.nb_streams() as _];
    let mut ost_time_bases = vec![Rational(0, 0); ictx.nb_streams() as _];
    let mut transcoders = HashMap::new();
    let mut ost_index = 0;

    for (ist_index, ist) in ictx.streams().enumerate() {
        let ist_medium = ist.parameters().medium();
        if ist_medium != media::Type::Audio && ist_medium != media::Type::Video {
            stream_mapping[ist_index] = -1;
            continue;
        }
        stream_mapping[ist_index] = ost_index;
        ist_time_bases[ist_index] = ist.time_base();
        if ist_medium == media::Type::Video {
            transcoders.insert(
                ist_index,
                Transcoder::new(
                    &ist,
                    &mut octx,
                    ost_index as _,
                    x264_opts.to_owned(),
                    Some(ist_index) == best_video_stream_index,
                )
                .unwrap(),
            );
        } else {
            let mut ost = octx.add_stream(encoder::find(codec::Id::None)).unwrap();
            ost.set_parameters(ist.parameters());
            unsafe {
                (*ost.parameters().as_mut_ptr()).codec_tag = 0;
            }
            continue;
        }
        ost_index += 1;
    }

    octx.set_metadata(ictx.metadata().to_owned());
    format::context::output::dump(&octx, 0, Some(&output_file));
    octx.write_header().unwrap();

    for (ost_index, _) in octx.streams().enumerate() {
        ost_time_bases[ost_index] = octx.stream(ost_index as _).unwrap().time_base();
    }

    for (stream, mut packet) in ictx.packets() {
        let ist_index = stream.index();
        let ost_index = stream_mapping[ist_index];
        if ost_index < 0 {
            continue;
        }
        let ost_time_base = ost_time_bases[ost_index as usize];
        match transcoders.get_mut(&ist_index) {
            Some(transcoder) => {
                transcoder.send_packet_to_decoder(&packet);
                transcoder.receive_and_process_decoded_frames(&mut octx, ost_time_base);
            }
            None => {
                packet.rescale_ts(ist_time_bases[ist_index], ost_time_base);
                packet.set_position(-1);
                packet.set_stream(ost_index as _);
                packet.write_interleaved(&mut octx).unwrap();
            }
        }
    }

    for (ost_index, transcoder) in transcoders.iter_mut() {
        let ost_time_base = ost_time_bases[*ost_index];
        transcoder.send_eof_to_decoder();
        transcoder.receive_and_process_decoded_frames(&mut octx, ost_time_base);
        transcoder.send_eof_to_encoder();
        transcoder.receive_and_process_encoded_packets(&mut octx, ost_time_base);
    }

    octx.write_trailer().unwrap();
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    pub async fn test() {
        let input_file =
            "downloads/狼与香辛料 行商邂逅贤狼(3330)/狼与香辛料 行商邂逅贤狼 - 1 - LoliHouse.mkv"
                .to_string();
        let _t = trans_mkv_2_mp4(&input_file).await.unwrap();
    }
}
