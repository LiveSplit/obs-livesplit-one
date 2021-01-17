# <img src="https://raw.githubusercontent.com/LiveSplit/LiveSplit/master/LiveSplit/Resources/Icon.png" alt="LiveSplit" height="42" width="45" align="top"/> obs-livesplit-one

A plugin for OBS Studio that allows adding LiveSplit One as a source.

## How to install

Download the latest release for your operating system from the
[Releases](https://github.com/CryZe/obs-livesplit-one/releases).

### Windows

- Extract the `obs-livesplit-one.dll` to `C:\Program Files
(x86)\obs-studio\obs-plugins\64bit` or equivalent install directory.

### Linux

- `mkdir -p $HOME/.config/obs-studio/plugins`
- Untar, e.g.: `tar -zxvf obs-livesplit-one-v0.0.1-x86_64-unknown-linux-gnu.tar.gz -C
   $HOME/.config/obs-studio/plugins/`

## Developer Notes

If you add a new function to `ffi.rs`, also add it to `exports.def` and run
(needs to be done in the Visual Studio console):
```sh
lib /def:exports.def /OUT:obs.lib /MACHINE:x64
```
