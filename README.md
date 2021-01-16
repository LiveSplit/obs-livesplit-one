# <img src="https://raw.githubusercontent.com/LiveSplit/LiveSplit/master/LiveSplit/Resources/Icon.png" alt="LiveSplit" height="42" width="45" align="top"/> obs-livesplit-one

A plugin for OBS Studio that allows adding LiveSplit One as a source.

## How to install

### Windows

Extract the `obs-livesplit-one.dll` to `C:\Program Files
(x86)\obs-studio\obs-plugins\64bit` or equivalent install directory.

### Linux

Extract the `libobs_livesplit_one.so` as
`$HOME/.config/obs-studio/plugins/livesplit-one/bin/64bit/libobs_livesplit_one.so`
Create all missing folders.

## Developer Notes

If you add a new function to `ffi.rs`, also add it to `exports.def` and run
(needs to be done in the Visual Studio console):
```sh
lib /def:exports.def /OUT:obs.lib /MACHINE:x64
```
