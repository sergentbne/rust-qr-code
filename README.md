## this is a work in progress, expect bugs and problems ! 

# File To QRCODE Video.

This program is *supposed* to be quite easy to use!
**WARNING, This program is quite greedy in file size, so donc forget to do**
```bash
cargo clean
```
**from time to time.**
```bash
git clone
cargo run --release {name of file}
```


This will take the file, make a series of qrcode with the hell of the qrcode crate and produce a video with it. 

**update v1.2.0: it is now able to set input and output and framerate with **

```bash
cargo run --release  -- -i {input file} -o {output file} -f {desired framerate}
```

## Build instructions

### Windows 

Install ffmpeg through vcpkg, I swear it will save you lots of trouble.
```powershell
winget install vcpkg
vcpkg install ffmpeg llvm
cargo run --release -- {parameters}
```

### Macos

Uhhhhhhh probably do the same? This program was mostly done with Macos but I haven't tried on linux
```bash
brew install pkg-config rust ffmpeg
cargo run --release  -- {parameters}
```

### Linux 
***FULLY UNTESTED, TRY AT YOUR OWN RISK***
```bash
sudo apt install ffmpeg pkg-config
or
sudo pacman -Syu ffmpeg pkg-config
```






