use crate::api::do_anime_task::handle_error;
use std::fs;
use anyhow::Error;
use ffmpeg::{codec, encoder, format, media};
use ffmpeg_next as ffmpeg;
use log;
use rsubs_lib::ssa;
use rsubs_lib::vtt;

#[derive(Debug)]
pub struct SubtitleInfo {
    pub index: usize,
    pub title: String,
    pub format: String,
}

#[allow(dead_code)]
pub async fn get_subtitle_info(path: &String) -> Result<Vec<SubtitleInfo>, Error> {
    ffmpeg::init().unwrap();

    let ictx = format::input(path).unwrap();
    let mut subtitle_vec: Vec<SubtitleInfo> = vec![];

    for (ist_index, ist) in ictx.streams().enumerate() {
        let ist_medium = ist.parameters().medium();
        if ist_medium == media::Type::Subtitle {
            subtitle_vec.push(SubtitleInfo {
                index: ist_index,
                title: ist.metadata().get("title").unwrap().to_string(),
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
    for subtitle in subtitle_vec {
        let output_file = format!("{} - {}.{}", video_name, subtitle.title, subtitle.format);
        if let Ok(_) = extract_subtitle_handler(subtitle.index, &path, &output_file).await {
            let vtt_path = output_file.split(".").next().unwrap().to_string() + ".vtt";
            if let Ok(_) = trans_subtitle_to_vtt(&output_file, &vtt_path).await {
                let subtitle_name = vtt_path.split("/").last().unwrap().to_string();
                output_subtitle_file.push(subtitle_name);
                log::info!("Successfully extracted subtitle from {}", path);
            } else {
                log::warn!("Failed to trans format to vtt for [{}]", path);
                continue;
            }
            match fs::remove_dir(&output_file) {
                Ok(_) => continue,
                Err(_) => {
                    log::warn!("Failed to remove tmp file [{}]", output_file);
                    continue;
                }
            }
        } else {
            log::warn!("Failed to extract subtitle from {}", path);
            continue;
        }
    }

    Ok(output_subtitle_file)
}

#[allow(dead_code)]
async fn extract_subtitle_handler(
    subtitle_stream_index: usize,
    input_file: &String,
    output_file: &String,
) -> Result<(), Error> {
    ffmpeg::init().unwrap();

    let mut ictx = format::input(&input_file).unwrap();
    let mut octx = format::output(&output_file).unwrap();

    let ist = ictx.stream(subtitle_stream_index).unwrap();

    // 创建一个与输入流具有相同参数的输出流
    let ist_time_base = ist.time_base();
    let mut ost = octx.add_stream(encoder::find(codec::Id::None)).unwrap();
    ost.set_parameters(ist.parameters());
    unsafe {
        (*ost.parameters().as_mut_ptr()).codec_tag = 0;
    }

    // 复制元数据
    octx.set_metadata(ictx.metadata().to_owned());
    octx.write_header().unwrap();

    // 复制字幕流
    for (stream, mut packet) in ictx.packets() {
        if stream.index() == subtitle_stream_index {
            let ost = octx.stream(0).unwrap();
            // 重新映射时间戳
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
    vtt::VTTFile::from(ssa::parse(intput_file.to_string()).unwrap()) // Can read either a file or a string
        // converts file to WEBVTT
        .to_file(output_file) // Writes the converted subtitle to a file
        .unwrap();

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    pub async fn test_extract() {
        let input_file = "/Users/heason/Documents/anime/末班列车去哪里了？(3305)/末班列车去哪里了？ - 1 - LoliHouse.mkv".to_string();
        let t = extract_subtitle(&input_file).await.unwrap();
        println!("{:?}", t);
    }

    #[tokio::test]
    pub async fn test_trans() {
        let input_file = "/Users/heason/Documents/anime/1.ass".to_string();
        let output_file = "/Users/heason/Documents/anime/1.vtt".to_string();
        let _t = trans_subtitle_to_vtt(&input_file, &output_file)
            .await
            .unwrap();
        // println!("{:?}", t);
    }
}
