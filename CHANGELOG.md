# Changelog

**English** | [Español](CHANGELOG.es.md) | [日本語](CHANGELOG.ja.md)

All notable changes to KanniChan are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[Semantic Versioning](https://semver.org/).

## [0.3.0] - 2026-11-15

### Added

- Overlay mode: shows the subtitles on top of the screen, without OBS, for screen sharing on Discord. The screen they appear on is chosen in Settings.
- Automatic updates on Windows: KanniChan tells you when a new version is out and updates with one button. The download is verified before it is installed.
- This changelog, in English, Spanish and Japanese. Each version's release notes on GitHub come from it.

## [0.2.0] - 2026-11-14

### Added

- Configurable subtitle style: size, outline, text color, outline color and furigana. Changes show up in OBS right away.
- Voice detection with Silero VAD: claps, knocks and background noise no longer turn into text. New "Voice detection" setting.
- Configurable install folder for the components, in the setup wizard and in Settings.

## [0.1.0] - 2026-11-13

### Added

- Local Spanish ⇄ Japanese translator through Ollama, with Hunyuan-MT 7B or HY-MT 1.5 1.8B.
- Japanese always shown with furigana.
- Text to speech with VOICEVOX (Japanese, with a choice of voices) and Piper (Spanish).
- Streamer mode: speech recognition with Whisper, translation and subtitles for OBS as a browser source.
- Setup wizard for the dependencies on first launch and in Settings.
- Interface in Spanish, English and Japanese, following the system language.
- Windows builds for NVIDIA (CUDA) and AMD (Vulkan). The CUDA runtime is downloaded from NVIDIA instead of being bundled.

[Unreleased]: https://github.com/CrawKatt/KanniChan/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/CrawKatt/KanniChan/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/CrawKatt/KanniChan/releases/tag/v0.1.0
