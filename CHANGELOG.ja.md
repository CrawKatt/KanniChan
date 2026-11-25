# 変更履歴

[English](CHANGELOG.md) | [Español](CHANGELOG.es.md) | **日本語**

KanniChanの主な変更はすべてここに記録します。書式は
[Keep a Changelog](https://keepachangelog.com/ja/1.1.0/) に、バージョン番号は
[セマンティック バージョニング](https://semver.org/lang/ja/) に従います。

## [0.3.0] - 2026-11-15

### 追加

- オーバーレイモード：OBSを使わずに字幕を画面の上に表示します。Discordで画面共有するときに使えます。表示する画面は設定で選べます。
- Windowsでの自動アップデート：新しいバージョンが出るとKanniChanが知らせ、ボタンひとつで更新できます。ダウンロードはインストール前に検証されます。
- 英語・スペイン語・日本語のこの変更履歴。GitHubの各バージョンのリリースノートはここから作られます。

## [0.2.0] - 2026-11-14

### 追加

- 字幕のスタイルを設定可能に：サイズ、縁取り、文字の色、縁取りの色、ふりがな。変更はOBSにすぐ反映されます。
- Silero VADによる音声検出：拍手や物音、背景の雑音が文字にならなくなりました。新しい設定「音声検出」を追加。
- コンポーネントのインストール先を、セットアップと設定で選べるように。

## [0.1.0] - 2026-11-13

### 追加

- Ollamaを使ったローカルのスペイン語 ⇄ 日本語翻訳（Hunyuan-MT 7B または HY-MT 1.5 1.8B）。
- 日本語は常にふりがな付きで表示。
- 読み上げ：日本語はVOICEVOX（声を選択可能）、スペイン語はPiper。
- 配信モード：Whisperによる音声認識、翻訳、OBSのブラウザソース用の字幕。
- 初回起動時と設定画面での依存関係のセットアップ。
- システムの言語に合わせたスペイン語・英語・日本語のインターフェース。
- Windows版はNVIDIA（CUDA）とAMD（Vulkan）の2種類。CUDAランタイムは同梱せず、NVIDIAからダウンロードします。

[Unreleased]: https://github.com/CrawKatt/KanniChan/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/CrawKatt/KanniChan/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/CrawKatt/KanniChan/releases/tag/v0.1.0
