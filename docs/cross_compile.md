# 编译

## MacOS 交叉编译 x86_64 平台的 Windows 二进制文件
为避免 windows lib 对本地的 lib 造成干扰，这里使用 docker 编译 Windows 二进制文件

### 构建 build 镜像
#### 创建 ubutnu 容器
```
docker run -itd --net=host --name app-build ubuntu:22.04
docker exec -it app-build bash
```

#### 安装基础工具
```
apt update && apt install --reinstall ca-certificates
apt update && apt install -y build-essential libssl-dev libsqlite3-dev libclang-dev pkg-config libavcodec-dev libavdevice-dev libavfilter-dev libavformat-dev libavutil-dev libpostproc-dev libswresample-dev libswscale-dev curl
```

#### 安装 rust
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

#### 安装 diesel_cli
```
cargo install diesel_cli --no-default-features --features sqlite
```

#### 安装交叉编译工具链
```
apt install pkg-config mingw-w64
rustup target add x86_64-pc-windows-gnu
```

##### 打包镜像
```
docker commit -a "autoAnime" -m "create new img" [conatinerd id] build:0.1.1
```


### 编译 windows 依赖库

#### 安装交叉编译工具链
```
brew install pkg-config mingw-w64
```

#### ffmpeg
```
git clone https://git.ffmpeg.org/ffmpeg.git ffmpeg
cd ffmpeg
./configure \
    --arch=x86_64 \
    --target-os=mingw32 \
    --cross-prefix=x86_64-w64-mingw32- \
    --enable-cross-compile \
    --prefix=/path/to/output/ffmpeg-win
make
make instll
```

#### Sqlite3
```
wget https://www.sqlite.org/2024/sqlite-autoconf-3460000.tar.gz
tar -xf sqlite-autoconf-3460000.tar.gz
cd sqlite-autoconf-3460000
./configure \
		--host=x86_64-w64-mingw32 \
		--target=x86_64-w64-mingw32 \
		--build=aarch64-apple-darwin \
		--prefix=/path/to/output/sqlite3-win
make 
make install
```

#### openssl
```
wget https://www.openssl.org/source/openssl-3.0.14.tar.gz
tar -xf /openssl-3.0.14.tar.gz
cd openssl-3.0.14

export CC=x86_64-w64-mingw32-gcc
export CXX=x86_64-w64-mingw32-g++
export AR=x86_64-w64-mingw32-ar
export RANLIB=x86_64-w64-mingw32-ranlib
export WINDRES=x86_64-w64-mingw32-windres

./Configure mingw64
make
makedir /path/to/output/libssl3-win
make install DESTDIR=/path/to/output/libssl3-win
```

#### 修改 windows lib 中 pkg-config 的 .pc 文件

为了让编译器能够通过 pkg-config 获取到lib的位置，需要修改.pc文件中的路径。如 ffmpeg-win/lib/pkg-config/libavcodec.pc, 将 `prefix` 和 `libdir` , `includedir`修改如下：
```
prefix=/usr/src/ffmpeg
exec_prefix=${prefix}
libdir=/usr/src/ffmpeg/lib
includedir=/usr/src/ffmpeg/include

Name: libavcodec
Description: FFmpeg codec library
Version: 61.3.100
Requires: libswresample >= 5.1.100, libavutil >= 59.8.100
Requires.private: 
Conflicts:
Libs: -L${libdir}  -lavcodec -lm -latomic -lmfuuid -lole32 -lstrmiids -lole32 -luser32
Libs.private: 
Cflags: -I${includedir}
```
ffmpeg中的其余 .pc 文件与及 Sqlite3 和 openssl 中的 .pc 文件也按照相同方式修改即可。

### Docker 编译
#### 下载代码
```
git clone https://github.com/FortyWinters/autoAnimeR.git
```

#### 启动 build 镜像
```
docker run -itd --name build \
		-v /path/to/ffmpeg-win:/usr/src/ffmpeg \
		-v /path/to/sqlite3-win:/usr/src/sqlite3 \
		-v /path/to/openssl-win:/usr/src/openssl \
		-v autoAnimeR:/usr/src/autoAnimeR \
		build:0.1.1

docker exec -it build bash
```

#### 编译
```
export PKG_CONFIG_ALLOW_CROSS=1
export PKG_CONFIG_PATH=/usr/src/ffmpeg/lib/pkgconfig:/usr/src/sqlite3/lib/pkgconfig:/usr/src/openssl/lib64/pkgconfig
export PKG_CONFIG_LIBDIR=/usr/src/ffmpeg/lib/pkgconfig:/usr/src/sqlite3/lib/pkgconfig:/usr/src/openssl/lib64/pkgconfig
export LIBRARY_PATH="/usr/src/ffmpeg/lib:/usr/src/sqlite3/lib:/usr/src/openssl/lib64"
export CFLAGS="-I/usr/src/ffmpeg/include -I/usr/src/sqlite3/include -I/usr/src/openssl/include/openssl"
export LDFLAGS="-L/usr/src/ffmpeg/lib -L/usr/src/sqlite3/lib -L/usr/src/openssl/lib64"

cd /usr/src/autoAnimeR/app
cargo build --target x86_64-pc-windows-gnu
```

#### Done
在宿主机的 `./autoAnimeR/app/target/x86_64-pc-windows-gnu` 目录下可以找到编译生成的 app.exe 文件
####

## MacOS 交叉编译 Linux 二进制文件
TBD