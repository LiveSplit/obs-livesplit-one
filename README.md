# <img src="https://raw.githubusercontent.com/LiveSplit/LiveSplit/master/res/Icon.svg" alt="LiveSplit" height="42" width="45" align="top"/> obs-livesplit-one

A plugin for OBS Studio that allows adding LiveSplit One as a source.

## How to install

Download the latest release for your operating system from the
[Releases](https://github.com/LiveSplit/obs-livesplit-one/releases).

### Windows

- Extract the `obs-livesplit-one.dll` to `C:\Program Files\obs-studio\obs-plugins\64bit` or equivalent install directory.

### Linux

- `mkdir -p $HOME/.config/obs-studio/plugins`
- Untar, e.g.: `tar -zxvf obs-livesplit-one-*-x86_64-unknown-linux-gnu.tar.gz -C $HOME/.config/obs-studio/plugins/`

#### Flatpak

- When using OBS from Flathub, this plugin can be installed with the command `flatpak install flathub com.obsproject.Studio.Plugin.OBSLivesplitOne`

### macOS

- Right click your `OBS` -> Options -> Show in Finder
- Right click the `OBS.app` -> Show Package Contents
- Drag `obs-livesplit-one.plugin` into `Contents/PlugIns`

## Usage

### Add a LiveSplit One source

Click the "add source" button, as usual, and choose _LiveSplit One_. In the
source's properties, you can choose a split file and a layout.

### Configure hotkeys

In ObS Studio's _Settings_ menu, under the _Hotkeys_ tab, scroll to the source's
name, where you can set hotkeys for the various actions.

### Add multiple sources with the same splits

If you add multiple sources that each use the same splits, but different
layouts, they all share the same state. This allows for a lot more complex
layouts than what is traditionally possible where you could for example show the
splits on a completely different part of your stream than the timer itself.
