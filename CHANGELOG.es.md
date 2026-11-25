# Changelog

[English](CHANGELOG.md) | **Español** | [日本語](CHANGELOG.ja.md)

Todos los cambios importantes de KanniChan se anotan aquí. El formato sigue
[Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y las versiones siguen
[Semantic Versioning](https://semver.org/lang/es/).

## [0.3.0] - 2026-11-15

### Añadido

- Modo Overlay: muestra los subtítulos encima de la pantalla, sin OBS, para compartir pantalla en Discord. En Ajustes se elige en qué pantalla aparecen.
- Actualizaciones automáticas en Windows: KanniChan avisa cuando hay una versión nueva y se actualiza con un botón. La descarga se verifica antes de instalarla.
- Este changelog, en inglés, español y japonés. Las notas de cada versión en GitHub salen de aquí.

## [0.2.0] - 2026-11-14

### Añadido

- Estilo de los subtítulos configurable: tamaño, contorno, color del texto, color del contorno y furigana. Los cambios se ven en OBS al momento.
- Detección de voz con Silero VAD: los aplausos, golpes y ruidos de fondo ya no se convierten en texto. Ajuste nuevo «Detección de voz».
- Carpeta de instalación de los componentes configurable, en el asistente y en Ajustes.

## [0.1.0] - 2026-11-13

### Añadido

- Traductor español ⇄ japonés local con Ollama, con Hunyuan-MT 7B o HY-MT 1.5 1.8B.
- Japonés siempre con furigana.
- Lectura en voz alta con VOICEVOX (japonés, voz configurable) y Piper (español).
- Modo Streamer: reconocimiento de voz con Whisper, traducción y subtítulos para OBS como fuente de navegador.
- Asistente de instalación de dependencias en el primer arranque y en Ajustes.
- Interfaz en español, inglés y japonés según el idioma del sistema.
- Versiones para Windows con NVIDIA (CUDA) y AMD (Vulkan). El runtime de CUDA se descarga desde NVIDIA en vez de ir incluido.

[Unreleased]: https://github.com/CrawKatt/KanniChan/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/CrawKatt/KanniChan/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/CrawKatt/KanniChan/releases/tag/v0.1.0
