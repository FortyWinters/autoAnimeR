use crate::api::do_anime_task::handle_error;
use anyhow::Error;
use ffmpeg::{codec, encoder, format, media, Rational};
use ffmpeg_next as ffmpeg;
use ffmpeg_next::subtitle::Rect;
use regex::Regex;
use rsubs_lib::ssa;
use rsubs_lib::vtt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, Write};
use std::path::Path;

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
    octx.write_header().unwrap();

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
    add_basic_subtitle_info(&file).unwrap();

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

#[allow(dead_code)]
fn add_basic_subtitle_info(mut file: &fs::File) -> Result<(), Error> {
    writeln!(file, "[Script Info]").expect("Failed to write to output file");
    writeln!(file, "; Font Subset: 53F530EY - HYXuanSong 65S")
        .expect("Failed to write to output file");
    writeln!(file, "; Font Subset: H51RRKUV - FZLanTingHei-DB-GBK")
        .expect("Failed to write to output file");
    writeln!(file, "; Font Subset: YT7S98V8 - FZYaSong-M-GBK")
        .expect("Failed to write to output file");
    writeln!(file, "; Font Subset: HENI15HU - Source Han Sans JP")
        .expect("Failed to write to output file");
    writeln!(file, "; Font Subset: HKABK2M7 - Source Han Sans CN")
        .expect("Failed to write to output file");
    writeln!(file, "; Font Subset: THPFH7R9 - FOT-Matisse Pro M")
        .expect("Failed to write to output file");
    writeln!(file, "; Font Subset: USPWBSGJ - FZYaSong-R-GBK")
        .expect("Failed to write to output file");
    writeln!(file, "; Font Subset: VRVYS57Y - Source Han Sans TW")
        .expect("Failed to write to output file");
    writeln!(file, "Title: Subtitle File").expect("Failed to write to output file");
    writeln!(file, "ScriptType: v4.00+").expect("Failed to write to output file");
    writeln!(file, "WrapStyle: 0").expect("Failed to write to output file");
    writeln!(file, "ScaledBorderAndShadow: yes").expect("Failed to write to output file");
    writeln!(file, "PlayResX: 1920").expect("Failed to write to output file");
    writeln!(file, "PlayResY: 1080").expect("Failed to write to output file");
    writeln!(file, "").expect("Failed to write to output file");

    writeln!(file, "[V4+ Styles]").expect("Failed to write to output file");
    writeln!(file, "Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding").expect("Failed to write to output file");
    writeln!(file, "Style: Default,H51RRKUV,65,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,2,0,2,10,10,15,1").expect("Failed to write to output file");
    writeln!(file, "Style: Top,H51RRKUV,60,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,2,0,8,10,10,15,1").expect("Failed to write to output file");
    writeln!(file, "Style: Screen,YT7S98V8,60,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,0,0,7,10,10,15,1").expect("Failed to write to output file");
    writeln!(file, "Style: Staff,53F530EY,53,&H00FFFFFF,&H000000FF,&H00000000,&H14000000,0,0,0,0,100,100,0,0,1,2.5,0,8,22,22,20,1").expect("Failed to write to output file");
    writeln!(file, "Style: Title,YT7S98V8,75,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,1,2,7,10,10,15,1").expect("Failed to write to output file");
    writeln!(file, "Style: OP_JP,HENI15HU,68,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,-1,0,0,0,100,100,2,0,1,2,0,2,10,10,45,1").expect("Failed to write to output file");
    writeln!(file, "Style: OP_CHS,HKABK2M7,70,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,-1,0,0,0,100,100,2,0,1,2,0,8,10,10,35,1").expect("Failed to write to output file");
    writeln!(file, "Style: ED_JP,THPFH7R9,50,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,0.8,1.5,2,10,10,15,1").expect("Failed to write to output file");
    writeln!(file, "Style: ED_CH,USPWBSGJ,48,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,2,0,1,0.8,1.5,8,10,10,25,1").expect("Failed to write to output file");
    writeln!(file, "").expect("Failed to write to output file");

    writeln!(file, "[Events]").expect("Failed to write to output file");
    writeln!(
        file,
        "Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text"
    )
    .expect("Failed to write to output file");
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

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    pub async fn test_get_subtitle_info() {
        let input_file = "".to_string();
        let t = get_subtitle_info(&input_file).await.unwrap();
        println!("{:?}", t);
    }

    #[tokio::test]
    pub async fn test_extract() {
        let input_file = "".to_string();
        let t = extract_subtitle(&input_file).await.unwrap();
        println!("{:?}", t);
    }

    #[tokio::test]
    pub async fn test_mp4() {
        let input_file = "".to_string();
        let output_file = "".to_string();
        let t = extract_mp4_subtitle(2 as usize, &input_file, &output_file)
            .await
            .unwrap();
        println!("{:?}", t);
    }
}
