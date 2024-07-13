# autoAnimeR
autoAnime in Rust 🦀️

## 项目介绍
一款基于 Rust 🦀️ & Docker & qBittorrent的自动追番项目
### 项目特点
- 以qbittorrent作为下载工具
- 基于Mikan进行番剧更新
- bangumi评分
- 基于Vue3的WebUI 
- 一键订阅新番 🏄🏻‍♂️
- 自动抓取更新 ⏱️
- 自动重命名 ✏️
- 自动提取字幕 📄
- 网页播放 📺
- 番剧搜索 🔍
- 下载进度可视 👓

### 
- 番剧列表：
![](docs/img/anime_list.jpg)

- 播放页面：
![](docs/img/video_page.jpg)

- 番剧页面：
![](docs/img/anime_detail.jpg)


## 快速开始

### MacOS

#### 安装 rust：
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

#### 安装依赖
```
brew install sqlite ffmpeg
cargo install diesel_cli --no-default-features --features sqlite
```

#### 编译
```
git clone https://github.com/FortyWinters/autoAnimeR.git
cd autoAnimeR/APP
sh db_init.sh
cargo run
```

### Linux

#### 安装依赖
```
sudo apt update && sudo apt install -y build-essential libssl-dev libsqlite3-dev libclang-dev pkg-config libavcodec-dev libavdevice-dev libavfilter-dev libavformat-dev libavutil-dev libpostproc-dev libswresample-dev libswscale-dev curl
```

#### 安装rust
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cargo install diesel_cli --no-default-features --features sqlite
```

#### 编译
```
git clone https://github.com/FortyWinters/autoAnimeR.git
cd autoAnimeR/APP
sh db_init.sh
cargo run
```

### Windows
等待后续维护

## WebUi
http://127.0.0.1:5173
