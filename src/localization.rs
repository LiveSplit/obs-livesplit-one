use std::{
    ffi::{c_char, CStr},
    sync::OnceLock,
};

use livesplit_core::Lang;

use crate::ffi::obs_get_locale;

pub fn lang() -> Lang {
    static LANG: OnceLock<Lang> = OnceLock::new();
    *LANG.get_or_init(|| {
        Lang::parse_locale(unsafe {
            CStr::from_ptr(obs_get_locale())
                .to_str()
                .unwrap_or_default()
        })
    })
}

macro_rules! cstr {
    ($f:literal) => {
        CStr::as_ptr($f)
    };
}

pub enum Text {
    HotkeySplit,
    HotkeyReset,
    HotkeyUndoSplit,
    HotkeySkipSplit,
    HotkeyPause,
    HotkeyUndoAllPauses,
    HotkeyPreviousComparison,
    HotkeyNextComparison,
    HotkeyToggleTimingMethod,
    PropertyWidth,
    PropertyHeight,
    PropertySplits,
    PropertySplitsFilter,
    PropertyAutoSave,
    PropertySaveSplits,
    PropertyLayout,
    PropertyLayoutFilter,
    PropertyAdvancedStartGameOptions,
    PropertyGamePath,
    PropertyGamePathFilter,
    PropertyGameArguments,
    PropertyWorkingDirectory,
    PropertyWorkingDirectoryFilter,
    PropertyGameEnvironmentVars,
    PropertyStartGame,
    AutoSplitterUseLocal,
    AutoSplitterLocalFile,
    AutoSplitterLocalFileFilter,
    AutoSplitterNoSplitsLoaded,
    AutoSplitterActivate,
    AutoSplitterDeactivate,
    AutoSplitterWebsite,
    AutoSplitterSettingsGroup,
    AutoSplitterIncompatible,
    AutoSplitterUnavailable,
}

impl Text {
    pub fn resolve(self, lang: Lang) -> *const c_char {
        match lang {
            Lang::English => resolve_english(self),
            Lang::Dutch => resolve_dutch(self),
            Lang::French => resolve_french(self),
            Lang::German => resolve_german(self),
            Lang::Italian => resolve_italian(self),
            Lang::Portuguese => resolve_portuguese(self),
            Lang::Polish => resolve_polish(self),
            Lang::Russian => resolve_russian(self),
            Lang::Spanish => resolve_spanish(self),
            Lang::BrazilianPortuguese => resolve_brazilian_portuguese(self),
            Lang::ChineseSimplified => resolve_chinese_simplified(self),
            Lang::ChineseTraditional => resolve_chinese_traditional(self),
            Lang::Japanese => resolve_japanese(self),
            Lang::Korean => resolve_korean(self),
        }
    }
}

fn resolve_english(text: Text) -> *const c_char {
    match text {
        Text::HotkeySplit => cstr!(c"Start / Split"),
        Text::HotkeyReset => cstr!(c"Reset"),
        Text::HotkeyUndoSplit => cstr!(c"Undo Split"),
        Text::HotkeySkipSplit => cstr!(c"Skip Split"),
        Text::HotkeyPause => cstr!(c"Pause"),
        Text::HotkeyUndoAllPauses => cstr!(c"Undo All Pauses"),
        Text::HotkeyPreviousComparison => cstr!(c"Previous Comparison"),
        Text::HotkeyNextComparison => cstr!(c"Next Comparison"),
        Text::HotkeyToggleTimingMethod => cstr!(c"Toggle Timing Method"),
        Text::PropertyWidth => cstr!(c"Width"),
        Text::PropertyHeight => cstr!(c"Height"),
        Text::PropertySplits => cstr!(c"Splits"),
        Text::PropertySplitsFilter => cstr!(c"LiveSplit Splits (*.lss)"),
        Text::PropertyAutoSave => cstr!(c"Save On Reset"),
        Text::PropertySaveSplits => cstr!(c"Save Splits"),
        Text::PropertyLayout => cstr!(c"Layout"),
        Text::PropertyLayoutFilter => cstr!(c"LiveSplit Layouts (*.lsl *.ls1l)"),
        Text::PropertyAdvancedStartGameOptions => cstr!(c"Advanced start game options"),
        Text::PropertyGamePath => cstr!(c"Game Path"),
        Text::PropertyGamePathFilter => cstr!(c"Executable files (*)"),
        Text::PropertyGameArguments => cstr!(c"Game Arguments"),
        Text::PropertyWorkingDirectory => cstr!(c"Working Directory"),
        Text::PropertyWorkingDirectoryFilter => cstr!(c"Directories"),
        Text::PropertyGameEnvironmentVars => cstr!(c"Game Environment Variables (KEY=VALUE)"),
        Text::PropertyStartGame => cstr!(c"Start Game"),
        Text::AutoSplitterUseLocal => cstr!(c"Use local auto splitter"),
        Text::AutoSplitterLocalFile => cstr!(c"Local Auto Splitter File"),
        Text::AutoSplitterLocalFileFilter => cstr!(c"LiveSplit One Auto Splitter (*.wasm)"),
        Text::AutoSplitterNoSplitsLoaded => cstr!(c"No splits loaded"),
        Text::AutoSplitterActivate => cstr!(c"Activate"),
        Text::AutoSplitterDeactivate => cstr!(c"Deactivate"),
        Text::AutoSplitterWebsite => cstr!(c"Website"),
        Text::AutoSplitterSettingsGroup => cstr!(c"Auto Splitter Settings"),
        Text::AutoSplitterIncompatible => {
            cstr!(c"This game's auto splitter is incompatible with LiveSplit One.")
        }
        Text::AutoSplitterUnavailable => cstr!(c"No auto splitter available for this game."),
    }
}

fn resolve_dutch(text: Text) -> *const c_char {
    match text {
        Text::HotkeySplit => cstr!(c"Start / Split"),
        Text::HotkeyReset => cstr!(c"Reset"),
        Text::HotkeyUndoSplit => cstr!(c"Split ongedaan maken"),
        Text::HotkeySkipSplit => cstr!(c"Split overslaan"),
        Text::HotkeyPause => cstr!(c"Pauze"),
        Text::HotkeyUndoAllPauses => cstr!(c"Alle pauzes ongedaan maken"),
        Text::HotkeyPreviousComparison => cstr!(c"Vorige vergelijking"),
        Text::HotkeyNextComparison => cstr!(c"Volgende vergelijking"),
        Text::HotkeyToggleTimingMethod => cstr!(c"Timingmethode wisselen"),
        Text::PropertyWidth => cstr!(c"Breedte"),
        Text::PropertyHeight => cstr!(c"Hoogte"),
        Text::PropertySplits => cstr!(c"Splits"),
        Text::PropertySplitsFilter => cstr!(c"LiveSplit Splits (*.lss)"),
        Text::PropertyAutoSave => cstr!(c"Opslaan bij reset"),
        Text::PropertySaveSplits => cstr!(c"Splits opslaan"),
        Text::PropertyLayout => cstr!(c"Lay-out"),
        Text::PropertyLayoutFilter => cstr!(c"LiveSplit-lay-outs (*.lsl *.ls1l)"),
        Text::PropertyAdvancedStartGameOptions => cstr!(c"Geavanceerde startspelopties"),
        Text::PropertyGamePath => cstr!(c"Spelpad"),
        Text::PropertyGamePathFilter => cstr!(c"Uitvoerbare bestanden (*)"),
        Text::PropertyGameArguments => cstr!(c"Spelargumenten"),
        Text::PropertyWorkingDirectory => cstr!(c"Werkmap"),
        Text::PropertyWorkingDirectoryFilter => cstr!(c"Mappen"),
        Text::PropertyGameEnvironmentVars => {
            cstr!(c"Omgevingsvariabelen voor spel (KEY=VALUE)")
        }
        Text::PropertyStartGame => cstr!(c"Spel starten"),
        Text::AutoSplitterUseLocal => cstr!(c"Lokale auto-splitter gebruiken"),
        Text::AutoSplitterLocalFile => cstr!(c"Lokale auto-splitterbestand"),
        Text::AutoSplitterLocalFileFilter => cstr!(c"LiveSplit One Auto Splitter (*.wasm)"),
        Text::AutoSplitterNoSplitsLoaded => cstr!(c"Geen splits geladen"),
        Text::AutoSplitterActivate => cstr!(c"Activeren"),
        Text::AutoSplitterDeactivate => cstr!(c"Deactiveren"),
        Text::AutoSplitterWebsite => cstr!(c"Website"),
        Text::AutoSplitterSettingsGroup => cstr!(c"Auto-splitterinstellingen"),
        Text::AutoSplitterIncompatible => {
            cstr!(c"De auto-splitter van dit spel is niet compatibel met LiveSplit One.")
        }
        Text::AutoSplitterUnavailable => cstr!(c"Geen auto-splitter beschikbaar voor dit spel."),
    }
}

fn resolve_french(text: Text) -> *const c_char {
    match text {
        Text::HotkeySplit => cstr!(c"Démarrer / Split"),
        Text::HotkeyReset => cstr!(c"Réinitialiser"),
        Text::HotkeyUndoSplit => cstr!(c"Annuler le split"),
        Text::HotkeySkipSplit => cstr!(c"Passer le split"),
        Text::HotkeyPause => cstr!(c"Pause"),
        Text::HotkeyUndoAllPauses => cstr!(c"Annuler toutes les pauses"),
        Text::HotkeyPreviousComparison => cstr!(c"Comparaison précédente"),
        Text::HotkeyNextComparison => cstr!(c"Comparaison suivante"),
        Text::HotkeyToggleTimingMethod => cstr!(c"Basculer la méthode de timing"),
        Text::PropertyWidth => cstr!(c"Largeur"),
        Text::PropertyHeight => cstr!(c"Hauteur"),
        Text::PropertySplits => cstr!(c"Splits"),
        Text::PropertySplitsFilter => cstr!(c"Splits LiveSplit (*.lss)"),
        Text::PropertyAutoSave => cstr!(c"Enregistrer lors de la réinitialisation"),
        Text::PropertySaveSplits => cstr!(c"Enregistrer les splits"),
        Text::PropertyLayout => cstr!(c"Disposition"),
        Text::PropertyLayoutFilter => cstr!(c"Dispositions LiveSplit (*.lsl *.ls1l)"),
        Text::PropertyAdvancedStartGameOptions => {
            cstr!(c"Options avancées de lancement du jeu")
        }
        Text::PropertyGamePath => cstr!(c"Chemin du jeu"),
        Text::PropertyGamePathFilter => cstr!(c"Fichiers exécutables (*)"),
        Text::PropertyGameArguments => cstr!(c"Arguments du jeu"),
        Text::PropertyWorkingDirectory => cstr!(c"Répertoire de travail"),
        Text::PropertyWorkingDirectoryFilter => cstr!(c"Répertoires"),
        Text::PropertyGameEnvironmentVars => {
            cstr!(c"Variables d'environnement du jeu (KEY=VALUE)")
        }
        Text::PropertyStartGame => cstr!(c"Lancer le jeu"),
        Text::AutoSplitterUseLocal => cstr!(c"Utiliser un auto-splitter local"),
        Text::AutoSplitterLocalFile => cstr!(c"Fichier d'auto-splitter local"),
        Text::AutoSplitterLocalFileFilter => cstr!(c"LiveSplit One Auto Splitter (*.wasm)"),
        Text::AutoSplitterNoSplitsLoaded => cstr!(c"Aucun segment chargé"),
        Text::AutoSplitterActivate => cstr!(c"Activer"),
        Text::AutoSplitterDeactivate => cstr!(c"Désactiver"),
        Text::AutoSplitterWebsite => cstr!(c"Site web"),
        Text::AutoSplitterSettingsGroup => cstr!(c"Paramètres d'auto-splitter"),
        Text::AutoSplitterIncompatible => {
            cstr!(c"L'auto-splitter de ce jeu est incompatible avec LiveSplit One.")
        }
        Text::AutoSplitterUnavailable => cstr!(c"Aucun auto-splitter disponible pour ce jeu."),
    }
}

fn resolve_german(text: Text) -> *const c_char {
    match text {
        Text::HotkeySplit => cstr!(c"Start / Split"),
        Text::HotkeyReset => cstr!(c"Zurücksetzen"),
        Text::HotkeyUndoSplit => cstr!(c"Split rückgängig"),
        Text::HotkeySkipSplit => cstr!(c"Split überspringen"),
        Text::HotkeyPause => cstr!(c"Pause"),
        Text::HotkeyUndoAllPauses => cstr!(c"Alle Pausen rückgängig"),
        Text::HotkeyPreviousComparison => cstr!(c"Vorheriger Vergleich"),
        Text::HotkeyNextComparison => cstr!(c"Nächster Vergleich"),
        Text::HotkeyToggleTimingMethod => cstr!(c"Zeitmessmethode umschalten"),
        Text::PropertyWidth => cstr!(c"Breite"),
        Text::PropertyHeight => cstr!(c"Höhe"),
        Text::PropertySplits => cstr!(c"Splits"),
        Text::PropertySplitsFilter => cstr!(c"LiveSplit Splits (*.lss)"),
        Text::PropertyAutoSave => cstr!(c"Beim Zurücksetzen speichern"),
        Text::PropertySaveSplits => cstr!(c"Splits speichern"),
        Text::PropertyLayout => cstr!(c"Layout"),
        Text::PropertyLayoutFilter => cstr!(c"LiveSplit Layouts (*.lsl *.ls1l)"),
        Text::PropertyAdvancedStartGameOptions => cstr!(c"Erweiterte Startoptionen für das Spiel"),
        Text::PropertyGamePath => cstr!(c"Spielpfad"),
        Text::PropertyGamePathFilter => cstr!(c"Ausführbare Dateien (*)"),
        Text::PropertyGameArguments => cstr!(c"Spielargumente"),
        Text::PropertyWorkingDirectory => cstr!(c"Arbeitsverzeichnis"),
        Text::PropertyWorkingDirectoryFilter => cstr!(c"Verzeichnisse"),
        Text::PropertyGameEnvironmentVars => cstr!(c"Spiel-Umgebungsvariablen (KEY=VALUE)"),
        Text::PropertyStartGame => cstr!(c"Spiel starten"),
        Text::AutoSplitterUseLocal => cstr!(c"Lokalen Auto-Splitter verwenden"),
        Text::AutoSplitterLocalFile => cstr!(c"Lokale Auto-Splitter-Datei"),
        Text::AutoSplitterLocalFileFilter => cstr!(c"LiveSplit One Auto Splitter (*.wasm)"),
        Text::AutoSplitterNoSplitsLoaded => cstr!(c"Keine Splits geladen"),
        Text::AutoSplitterActivate => cstr!(c"Aktivieren"),
        Text::AutoSplitterDeactivate => cstr!(c"Deaktivieren"),
        Text::AutoSplitterWebsite => cstr!(c"Website"),
        Text::AutoSplitterSettingsGroup => cstr!(c"Auto-Splitter-Einstellungen"),
        Text::AutoSplitterIncompatible => {
            cstr!(c"Der Auto-Splitter dieses Spiels ist nicht mit LiveSplit One kompatibel.")
        }
        Text::AutoSplitterUnavailable => {
            cstr!(c"Für dieses Spiel ist kein Auto-Splitter verfügbar.")
        }
    }
}

fn resolve_italian(text: Text) -> *const c_char {
    match text {
        Text::HotkeySplit => cstr!(c"Avvia / Split"),
        Text::HotkeyReset => cstr!(c"Reimposta"),
        Text::HotkeyUndoSplit => cstr!(c"Annulla split"),
        Text::HotkeySkipSplit => cstr!(c"Salta split"),
        Text::HotkeyPause => cstr!(c"Pausa"),
        Text::HotkeyUndoAllPauses => cstr!(c"Annulla tutte le pause"),
        Text::HotkeyPreviousComparison => cstr!(c"Confronto precedente"),
        Text::HotkeyNextComparison => cstr!(c"Confronto successivo"),
        Text::HotkeyToggleTimingMethod => cstr!(c"Cambia metodo di cronometraggio"),
        Text::PropertyWidth => cstr!(c"Larghezza"),
        Text::PropertyHeight => cstr!(c"Altezza"),
        Text::PropertySplits => cstr!(c"Splits"),
        Text::PropertySplitsFilter => cstr!(c"LiveSplit Splits (*.lss)"),
        Text::PropertyAutoSave => cstr!(c"Salva al reset"),
        Text::PropertySaveSplits => cstr!(c"Salva splits"),
        Text::PropertyLayout => cstr!(c"Layout"),
        Text::PropertyLayoutFilter => cstr!(c"LiveSplit Layouts (*.lsl *.ls1l)"),
        Text::PropertyAdvancedStartGameOptions => cstr!(c"Opzioni avanzate di avvio del gioco"),
        Text::PropertyGamePath => cstr!(c"Percorso del gioco"),
        Text::PropertyGamePathFilter => cstr!(c"File eseguibili (*)"),
        Text::PropertyGameArguments => cstr!(c"Argomenti del gioco"),
        Text::PropertyWorkingDirectory => cstr!(c"Directory di lavoro"),
        Text::PropertyWorkingDirectoryFilter => cstr!(c"Directory"),
        Text::PropertyGameEnvironmentVars => cstr!(c"Variabili d'ambiente del gioco (KEY=VALUE)"),
        Text::PropertyStartGame => cstr!(c"Avvia gioco"),
        Text::AutoSplitterUseLocal => cstr!(c"Usa auto-splitter locale"),
        Text::AutoSplitterLocalFile => cstr!(c"File auto-splitter locale"),
        Text::AutoSplitterLocalFileFilter => cstr!(c"LiveSplit One Auto Splitter (*.wasm)"),
        Text::AutoSplitterNoSplitsLoaded => cstr!(c"Nessuno split caricato"),
        Text::AutoSplitterActivate => cstr!(c"Attiva"),
        Text::AutoSplitterDeactivate => cstr!(c"Disattiva"),
        Text::AutoSplitterWebsite => cstr!(c"Sito web"),
        Text::AutoSplitterSettingsGroup => cstr!(c"Impostazioni auto-splitter"),
        Text::AutoSplitterIncompatible => {
            cstr!(c"L'auto-splitter di questo gioco è incompatibile con LiveSplit One.")
        }
        Text::AutoSplitterUnavailable => {
            cstr!(c"Nessun auto-splitter disponibile per questo gioco.")
        }
    }
}

fn resolve_portuguese(text: Text) -> *const c_char {
    match text {
        Text::HotkeySplit => cstr!(c"Iniciar / Split"),
        Text::HotkeyReset => cstr!(c"Reiniciar"),
        Text::HotkeyUndoSplit => cstr!(c"Desfazer split"),
        Text::HotkeySkipSplit => cstr!(c"Ignorar split"),
        Text::HotkeyPause => cstr!(c"Pausar"),
        Text::HotkeyUndoAllPauses => cstr!(c"Desfazer todas as pausas"),
        Text::HotkeyPreviousComparison => cstr!(c"Comparação anterior"),
        Text::HotkeyNextComparison => cstr!(c"Próxima comparação"),
        Text::HotkeyToggleTimingMethod => cstr!(c"Alternar método de cronometragem"),
        Text::PropertyWidth => cstr!(c"Largura"),
        Text::PropertyHeight => cstr!(c"Altura"),
        Text::PropertySplits => cstr!(c"Splits"),
        Text::PropertySplitsFilter => cstr!(c"LiveSplit Splits (*.lss)"),
        Text::PropertyAutoSave => cstr!(c"Guardar ao reiniciar"),
        Text::PropertySaveSplits => cstr!(c"Guardar splits"),
        Text::PropertyLayout => cstr!(c"Layout"),
        Text::PropertyLayoutFilter => cstr!(c"Layouts do LiveSplit (*.lsl *.ls1l)"),
        Text::PropertyAdvancedStartGameOptions => cstr!(c"Opções avançadas de início do jogo"),
        Text::PropertyGamePath => cstr!(c"Caminho do jogo"),
        Text::PropertyGamePathFilter => cstr!(c"Ficheiros executáveis (*)"),
        Text::PropertyGameArguments => cstr!(c"Argumentos do jogo"),
        Text::PropertyWorkingDirectory => cstr!(c"Diretório de trabalho"),
        Text::PropertyWorkingDirectoryFilter => cstr!(c"Diretórios"),
        Text::PropertyGameEnvironmentVars => cstr!(c"Variáveis de ambiente do jogo (KEY=VALUE)"),
        Text::PropertyStartGame => cstr!(c"Iniciar jogo"),
        Text::AutoSplitterUseLocal => cstr!(c"Usar auto-splitter local"),
        Text::AutoSplitterLocalFile => cstr!(c"Ficheiro de auto-splitter local"),
        Text::AutoSplitterLocalFileFilter => cstr!(c"LiveSplit One Auto Splitter (*.wasm)"),
        Text::AutoSplitterNoSplitsLoaded => cstr!(c"Nenhum split carregado"),
        Text::AutoSplitterActivate => cstr!(c"Ativar"),
        Text::AutoSplitterDeactivate => cstr!(c"Desativar"),
        Text::AutoSplitterWebsite => cstr!(c"Website"),
        Text::AutoSplitterSettingsGroup => cstr!(c"Definições do auto-splitter"),
        Text::AutoSplitterIncompatible => {
            cstr!(c"O auto-splitter deste jogo é incompatível com o LiveSplit One.")
        }
        Text::AutoSplitterUnavailable => cstr!(c"Não há auto-splitter disponível para este jogo."),
    }
}

fn resolve_polish(text: Text) -> *const c_char {
    match text {
        Text::HotkeySplit => cstr!(c"Start / Split"),
        Text::HotkeyReset => cstr!(c"Reset"),
        Text::HotkeyUndoSplit => cstr!(c"Cofnij split"),
        Text::HotkeySkipSplit => cstr!(c"Pomiń split"),
        Text::HotkeyPause => cstr!(c"Pauza"),
        Text::HotkeyUndoAllPauses => cstr!(c"Cofnij wszystkie pauzy"),
        Text::HotkeyPreviousComparison => cstr!(c"Poprzednie porównanie"),
        Text::HotkeyNextComparison => cstr!(c"Następne porównanie"),
        Text::HotkeyToggleTimingMethod => cstr!(c"Przełącz metodę pomiaru czasu"),
        Text::PropertyWidth => cstr!(c"Szerokość"),
        Text::PropertyHeight => cstr!(c"Wysokość"),
        Text::PropertySplits => cstr!(c"Splity"),
        Text::PropertySplitsFilter => cstr!(c"LiveSplit Splity (*.lss)"),
        Text::PropertyAutoSave => cstr!(c"Zapisuj przy resecie"),
        Text::PropertySaveSplits => cstr!(c"Zapisz splity"),
        Text::PropertyLayout => cstr!(c"Układ"),
        Text::PropertyLayoutFilter => cstr!(c"Układy LiveSplit (*.lsl *.ls1l)"),
        Text::PropertyAdvancedStartGameOptions => cstr!(c"Zaawansowane opcje uruchamiania gry"),
        Text::PropertyGamePath => cstr!(c"Ścieżka gry"),
        Text::PropertyGamePathFilter => cstr!(c"Pliki wykonywalne (*)"),
        Text::PropertyGameArguments => cstr!(c"Argumenty gry"),
        Text::PropertyWorkingDirectory => cstr!(c"Katalog roboczy"),
        Text::PropertyWorkingDirectoryFilter => cstr!(c"Katalogi"),
        Text::PropertyGameEnvironmentVars => cstr!(c"Zmienne środowiskowe gry (KEY=VALUE)"),
        Text::PropertyStartGame => cstr!(c"Uruchom grę"),
        Text::AutoSplitterUseLocal => cstr!(c"Użyj lokalnego auto-splittera"),
        Text::AutoSplitterLocalFile => cstr!(c"Plik lokalnego auto-splittera"),
        Text::AutoSplitterLocalFileFilter => cstr!(c"LiveSplit One Auto Splitter (*.wasm)"),
        Text::AutoSplitterNoSplitsLoaded => cstr!(c"Nie wczytano splitów"),
        Text::AutoSplitterActivate => cstr!(c"Aktywuj"),
        Text::AutoSplitterDeactivate => cstr!(c"Dezaktywuj"),
        Text::AutoSplitterWebsite => cstr!(c"Strona internetowa"),
        Text::AutoSplitterSettingsGroup => cstr!(c"Ustawienia auto-splittera"),
        Text::AutoSplitterIncompatible => {
            cstr!(c"Auto-splitter tej gry jest niezgodny z LiveSplit One.")
        }
        Text::AutoSplitterUnavailable => cstr!(c"Brak auto-splittera dla tej gry."),
    }
}

fn resolve_russian(text: Text) -> *const c_char {
    match text {
        Text::HotkeySplit => cstr!(c"Старт / Сплит"),
        Text::HotkeyReset => cstr!(c"Сброс"),
        Text::HotkeyUndoSplit => cstr!(c"Отменить сплит"),
        Text::HotkeySkipSplit => cstr!(c"Пропустить сплит"),
        Text::HotkeyPause => cstr!(c"Пауза"),
        Text::HotkeyUndoAllPauses => cstr!(c"Отменить все паузы"),
        Text::HotkeyPreviousComparison => cstr!(c"Предыдущее сравнение"),
        Text::HotkeyNextComparison => cstr!(c"Следующее сравнение"),
        Text::HotkeyToggleTimingMethod => cstr!(c"Переключить метод тайминга"),
        Text::PropertyWidth => cstr!(c"Ширина"),
        Text::PropertyHeight => cstr!(c"Высота"),
        Text::PropertySplits => cstr!(c"Сплиты"),
        Text::PropertySplitsFilter => cstr!(c"Сплиты LiveSplit (*.lss)"),
        Text::PropertyAutoSave => cstr!(c"Сохранять при сбросе"),
        Text::PropertySaveSplits => cstr!(c"Сохранить сплиты"),
        Text::PropertyLayout => cstr!(c"Макет"),
        Text::PropertyLayoutFilter => cstr!(c"Макеты LiveSplit (*.lsl *.ls1l)"),
        Text::PropertyAdvancedStartGameOptions => cstr!(c"Расширенные параметры запуска игры"),
        Text::PropertyGamePath => cstr!(c"Путь к игре"),
        Text::PropertyGamePathFilter => cstr!(c"Исполняемые файлы (*)"),
        Text::PropertyGameArguments => cstr!(c"Аргументы игры"),
        Text::PropertyWorkingDirectory => cstr!(c"Рабочая папка"),
        Text::PropertyWorkingDirectoryFilter => cstr!(c"Папки"),
        Text::PropertyGameEnvironmentVars => cstr!(c"Переменные окружения игры (KEY=VALUE)"),
        Text::PropertyStartGame => cstr!(c"Запустить игру"),
        Text::AutoSplitterUseLocal => cstr!(c"Использовать локальный авто-сплиттер"),
        Text::AutoSplitterLocalFile => cstr!(c"Файл локального авто-сплиттера"),
        Text::AutoSplitterLocalFileFilter => cstr!(c"LiveSplit One Auto Splitter (*.wasm)"),
        Text::AutoSplitterNoSplitsLoaded => cstr!(c"Сплиты не загружены"),
        Text::AutoSplitterActivate => cstr!(c"Активировать"),
        Text::AutoSplitterDeactivate => cstr!(c"Деактивировать"),
        Text::AutoSplitterWebsite => cstr!(c"Веб-сайт"),
        Text::AutoSplitterSettingsGroup => cstr!(c"Настройки авто-сплиттера"),
        Text::AutoSplitterIncompatible => {
            cstr!(c"Авто-сплиттер этой игры несовместим с LiveSplit One.")
        }
        Text::AutoSplitterUnavailable => cstr!(c"Для этой игры нет авто-сплиттера."),
    }
}

fn resolve_spanish(text: Text) -> *const c_char {
    match text {
        Text::HotkeySplit => cstr!(c"Iniciar / Split"),
        Text::HotkeyReset => cstr!(c"Reiniciar"),
        Text::HotkeyUndoSplit => cstr!(c"Deshacer split"),
        Text::HotkeySkipSplit => cstr!(c"Omitir split"),
        Text::HotkeyPause => cstr!(c"Pausa"),
        Text::HotkeyUndoAllPauses => cstr!(c"Deshacer todas las pausas"),
        Text::HotkeyPreviousComparison => cstr!(c"Comparación anterior"),
        Text::HotkeyNextComparison => cstr!(c"Siguiente comparación"),
        Text::HotkeyToggleTimingMethod => cstr!(c"Alternar método de cronometraje"),
        Text::PropertyWidth => cstr!(c"Ancho"),
        Text::PropertyHeight => cstr!(c"Alto"),
        Text::PropertySplits => cstr!(c"Splits"),
        Text::PropertySplitsFilter => cstr!(c"Splits de LiveSplit (*.lss)"),
        Text::PropertyAutoSave => cstr!(c"Guardar al reiniciar"),
        Text::PropertySaveSplits => cstr!(c"Guardar splits"),
        Text::PropertyLayout => cstr!(c"Diseño"),
        Text::PropertyLayoutFilter => cstr!(c"Diseños de LiveSplit (*.lsl *.ls1l)"),
        Text::PropertyAdvancedStartGameOptions => cstr!(c"Opciones avanzadas de inicio del juego"),
        Text::PropertyGamePath => cstr!(c"Ruta del juego"),
        Text::PropertyGamePathFilter => cstr!(c"Archivos ejecutables (*)"),
        Text::PropertyGameArguments => cstr!(c"Argumentos del juego"),
        Text::PropertyWorkingDirectory => cstr!(c"Directorio de trabajo"),
        Text::PropertyWorkingDirectoryFilter => cstr!(c"Directorios"),
        Text::PropertyGameEnvironmentVars => cstr!(c"Variables de entorno del juego (KEY=VALUE)"),
        Text::PropertyStartGame => cstr!(c"Iniciar juego"),
        Text::AutoSplitterUseLocal => cstr!(c"Usar auto-splitter local"),
        Text::AutoSplitterLocalFile => cstr!(c"Archivo de auto-splitter local"),
        Text::AutoSplitterLocalFileFilter => cstr!(c"LiveSplit One Auto Splitter (*.wasm)"),
        Text::AutoSplitterNoSplitsLoaded => cstr!(c"No hay splits cargados"),
        Text::AutoSplitterActivate => cstr!(c"Activar"),
        Text::AutoSplitterDeactivate => cstr!(c"Desactivar"),
        Text::AutoSplitterWebsite => cstr!(c"Sitio web"),
        Text::AutoSplitterSettingsGroup => cstr!(c"Ajustes del auto-splitter"),
        Text::AutoSplitterIncompatible => {
            cstr!(c"El auto-splitter de este juego es incompatible con LiveSplit One.")
        }
        Text::AutoSplitterUnavailable => cstr!(c"No hay auto-splitter disponible para este juego."),
    }
}

fn resolve_brazilian_portuguese(text: Text) -> *const c_char {
    match text {
        Text::HotkeySplit => cstr!(c"Iniciar / Split"),
        Text::HotkeyReset => cstr!(c"Resetar"),
        Text::HotkeyUndoSplit => cstr!(c"Desfazer split"),
        Text::HotkeySkipSplit => cstr!(c"Pular split"),
        Text::HotkeyPause => cstr!(c"Pausar"),
        Text::HotkeyUndoAllPauses => cstr!(c"Desfazer todas as pausas"),
        Text::HotkeyPreviousComparison => cstr!(c"Comparação anterior"),
        Text::HotkeyNextComparison => cstr!(c"Próxima comparação"),
        Text::HotkeyToggleTimingMethod => cstr!(c"Alternar método de cronometragem"),
        Text::PropertyWidth => cstr!(c"Largura"),
        Text::PropertyHeight => cstr!(c"Altura"),
        Text::PropertySplits => cstr!(c"Splits"),
        Text::PropertySplitsFilter => cstr!(c"Splits do LiveSplit (*.lss)"),
        Text::PropertyAutoSave => cstr!(c"Salvar ao resetar"),
        Text::PropertySaveSplits => cstr!(c"Salvar splits"),
        Text::PropertyLayout => cstr!(c"Layout"),
        Text::PropertyLayoutFilter => cstr!(c"Layouts do LiveSplit (*.lsl *.ls1l)"),
        Text::PropertyAdvancedStartGameOptions => cstr!(c"Opções avançadas de início do jogo"),
        Text::PropertyGamePath => cstr!(c"Caminho do jogo"),
        Text::PropertyGamePathFilter => cstr!(c"Arquivos executáveis (*)"),
        Text::PropertyGameArguments => cstr!(c"Argumentos do jogo"),
        Text::PropertyWorkingDirectory => cstr!(c"Diretório de trabalho"),
        Text::PropertyWorkingDirectoryFilter => cstr!(c"Diretórios"),
        Text::PropertyGameEnvironmentVars => cstr!(c"Variáveis de ambiente do jogo (KEY=VALUE)"),
        Text::PropertyStartGame => cstr!(c"Iniciar jogo"),
        Text::AutoSplitterUseLocal => cstr!(c"Usar auto-splitter local"),
        Text::AutoSplitterLocalFile => cstr!(c"Arquivo de auto-splitter local"),
        Text::AutoSplitterLocalFileFilter => cstr!(c"LiveSplit One Auto Splitter (*.wasm)"),
        Text::AutoSplitterNoSplitsLoaded => cstr!(c"Nenhum split carregado"),
        Text::AutoSplitterActivate => cstr!(c"Ativar"),
        Text::AutoSplitterDeactivate => cstr!(c"Desativar"),
        Text::AutoSplitterWebsite => cstr!(c"Website"),
        Text::AutoSplitterSettingsGroup => cstr!(c"Configurações do auto-splitter"),
        Text::AutoSplitterIncompatible => {
            cstr!(c"O auto-splitter deste jogo é incompatível com o LiveSplit One.")
        }
        Text::AutoSplitterUnavailable => cstr!(c"Nenhum auto-splitter disponível para este jogo."),
    }
}

fn resolve_chinese_simplified(text: Text) -> *const c_char {
    match text {
        Text::HotkeySplit => cstr!(c"开始 / 分段"),
        Text::HotkeyReset => cstr!(c"重置"),
        Text::HotkeyUndoSplit => cstr!(c"撤销分段"),
        Text::HotkeySkipSplit => cstr!(c"跳过分段"),
        Text::HotkeyPause => cstr!(c"暂停"),
        Text::HotkeyUndoAllPauses => cstr!(c"撤销全部暂停"),
        Text::HotkeyPreviousComparison => cstr!(c"上一个比较"),
        Text::HotkeyNextComparison => cstr!(c"下一个比较"),
        Text::HotkeyToggleTimingMethod => cstr!(c"切换计时方式"),
        Text::PropertyWidth => cstr!(c"宽度"),
        Text::PropertyHeight => cstr!(c"高度"),
        Text::PropertySplits => cstr!(c"分段"),
        Text::PropertySplitsFilter => cstr!(c"LiveSplit 分段 (*.lss)"),
        Text::PropertyAutoSave => cstr!(c"重置时保存"),
        Text::PropertySaveSplits => cstr!(c"保存分段"),
        Text::PropertyLayout => cstr!(c"布局"),
        Text::PropertyLayoutFilter => cstr!(c"LiveSplit 布局 (*.lsl *.ls1l)"),
        Text::PropertyAdvancedStartGameOptions => cstr!(c"高级启动游戏选项"),
        Text::PropertyGamePath => cstr!(c"游戏路径"),
        Text::PropertyGamePathFilter => cstr!(c"可执行文件 (*)"),
        Text::PropertyGameArguments => cstr!(c"游戏参数"),
        Text::PropertyWorkingDirectory => cstr!(c"工作目录"),
        Text::PropertyWorkingDirectoryFilter => cstr!(c"目录"),
        Text::PropertyGameEnvironmentVars => cstr!(c"游戏环境变量 (KEY=VALUE)"),
        Text::PropertyStartGame => cstr!(c"启动游戏"),
        Text::AutoSplitterUseLocal => cstr!(c"使用本地自动分段器"),
        Text::AutoSplitterLocalFile => cstr!(c"本地自动分段器文件"),
        Text::AutoSplitterLocalFileFilter => cstr!(c"LiveSplit One 自动分段器 (*.wasm)"),
        Text::AutoSplitterNoSplitsLoaded => cstr!(c"未加载分段"),
        Text::AutoSplitterActivate => cstr!(c"启用"),
        Text::AutoSplitterDeactivate => cstr!(c"停用"),
        Text::AutoSplitterWebsite => cstr!(c"网站"),
        Text::AutoSplitterSettingsGroup => cstr!(c"自动分段器设置"),
        Text::AutoSplitterIncompatible => cstr!(c"该游戏的自动分段器与 LiveSplit One 不兼容。"),
        Text::AutoSplitterUnavailable => cstr!(c"此游戏没有可用的自动分段器。"),
    }
}

fn resolve_chinese_traditional(text: Text) -> *const c_char {
    match text {
        Text::HotkeySplit => cstr!(c"開始 / 分段"),
        Text::HotkeyReset => cstr!(c"重設"),
        Text::HotkeyUndoSplit => cstr!(c"撤銷分段"),
        Text::HotkeySkipSplit => cstr!(c"跳過分段"),
        Text::HotkeyPause => cstr!(c"暫停"),
        Text::HotkeyUndoAllPauses => cstr!(c"撤銷全部暫停"),
        Text::HotkeyPreviousComparison => cstr!(c"上一個比較"),
        Text::HotkeyNextComparison => cstr!(c"下一個比較"),
        Text::HotkeyToggleTimingMethod => cstr!(c"切換計時方式"),
        Text::PropertyWidth => cstr!(c"寬度"),
        Text::PropertyHeight => cstr!(c"高度"),
        Text::PropertySplits => cstr!(c"分段"),
        Text::PropertySplitsFilter => cstr!(c"LiveSplit 分段 (*.lss)"),
        Text::PropertyAutoSave => cstr!(c"重設時保存"),
        Text::PropertySaveSplits => cstr!(c"儲存分段"),
        Text::PropertyLayout => cstr!(c"版面配置"),
        Text::PropertyLayoutFilter => cstr!(c"LiveSplit 版面配置 (*.lsl *.ls1l)"),
        Text::PropertyAdvancedStartGameOptions => cstr!(c"進階啟動遊戲選項"),
        Text::PropertyGamePath => cstr!(c"遊戲路徑"),
        Text::PropertyGamePathFilter => cstr!(c"可執行檔 (*)"),
        Text::PropertyGameArguments => cstr!(c"遊戲參數"),
        Text::PropertyWorkingDirectory => cstr!(c"工作目錄"),
        Text::PropertyWorkingDirectoryFilter => cstr!(c"目錄"),
        Text::PropertyGameEnvironmentVars => cstr!(c"遊戲環境變數 (KEY=VALUE)"),
        Text::PropertyStartGame => cstr!(c"啟動遊戲"),
        Text::AutoSplitterUseLocal => cstr!(c"使用本機自動分段器"),
        Text::AutoSplitterLocalFile => cstr!(c"本機自動分段器檔案"),
        Text::AutoSplitterLocalFileFilter => cstr!(c"LiveSplit One 自動分段器 (*.wasm)"),
        Text::AutoSplitterNoSplitsLoaded => cstr!(c"未載入分段"),
        Text::AutoSplitterActivate => cstr!(c"啟用"),
        Text::AutoSplitterDeactivate => cstr!(c"停用"),
        Text::AutoSplitterWebsite => cstr!(c"網站"),
        Text::AutoSplitterSettingsGroup => cstr!(c"自動分段器設定"),
        Text::AutoSplitterIncompatible => cstr!(c"此遊戲的自動分段器與 LiveSplit One 不相容。"),
        Text::AutoSplitterUnavailable => cstr!(c"此遊戲沒有可用的自動分段器。"),
    }
}

fn resolve_japanese(text: Text) -> *const c_char {
    match text {
        Text::HotkeySplit => cstr!(c"開始 / スプリット"),
        Text::HotkeyReset => cstr!(c"リセット"),
        Text::HotkeyUndoSplit => cstr!(c"スプリットを取り消す"),
        Text::HotkeySkipSplit => cstr!(c"スプリットをスキップ"),
        Text::HotkeyPause => cstr!(c"ポーズ"),
        Text::HotkeyUndoAllPauses => cstr!(c"すべてのポーズを取り消す"),
        Text::HotkeyPreviousComparison => cstr!(c"前の比較対象"),
        Text::HotkeyNextComparison => cstr!(c"次の比較対象"),
        Text::HotkeyToggleTimingMethod => cstr!(c"計測方法を切り替え"),
        Text::PropertyWidth => cstr!(c"幅"),
        Text::PropertyHeight => cstr!(c"高さ"),
        Text::PropertySplits => cstr!(c"スプリット"),
        Text::PropertySplitsFilter => cstr!(c"LiveSplit スプリット (*.lss)"),
        Text::PropertyAutoSave => cstr!(c"リセット時に保存"),
        Text::PropertySaveSplits => cstr!(c"スプリットを保存"),
        Text::PropertyLayout => cstr!(c"レイアウト"),
        Text::PropertyLayoutFilter => cstr!(c"LiveSplit レイアウト (*.lsl *.ls1l)"),
        Text::PropertyAdvancedStartGameOptions => cstr!(c"ゲーム起動の詳細オプション"),
        Text::PropertyGamePath => cstr!(c"ゲームパス"),
        Text::PropertyGamePathFilter => cstr!(c"実行ファイル (*)"),
        Text::PropertyGameArguments => cstr!(c"ゲーム引数"),
        Text::PropertyWorkingDirectory => cstr!(c"作業ディレクトリ"),
        Text::PropertyWorkingDirectoryFilter => cstr!(c"ディレクトリ"),
        Text::PropertyGameEnvironmentVars => cstr!(c"ゲーム環境変数 (KEY=VALUE)"),
        Text::PropertyStartGame => cstr!(c"ゲームを開始"),
        Text::AutoSplitterUseLocal => cstr!(c"ローカル自動スプリッターを使用"),
        Text::AutoSplitterLocalFile => cstr!(c"ローカル自動スプリッターファイル"),
        Text::AutoSplitterLocalFileFilter => cstr!(c"LiveSplit One Auto Splitter (*.wasm)"),
        Text::AutoSplitterNoSplitsLoaded => cstr!(c"スプリットが読み込まれていません"),
        Text::AutoSplitterActivate => cstr!(c"有効化"),
        Text::AutoSplitterDeactivate => cstr!(c"無効化"),
        Text::AutoSplitterWebsite => cstr!(c"ウェブサイト"),
        Text::AutoSplitterSettingsGroup => cstr!(c"自動スプリッター設定"),
        Text::AutoSplitterIncompatible => {
            cstr!(c"このゲームの自動スプリッターは LiveSplit One と互換性がありません。")
        }
        Text::AutoSplitterUnavailable => {
            cstr!(c"このゲームで利用可能な自動スプリッターはありません。")
        }
    }
}

fn resolve_korean(text: Text) -> *const c_char {
    match text {
        Text::HotkeySplit => cstr!(c"시작 / 스플릿"),
        Text::HotkeyReset => cstr!(c"리셋"),
        Text::HotkeyUndoSplit => cstr!(c"스플릿 되돌리기"),
        Text::HotkeySkipSplit => cstr!(c"스플릿 건너뛰기"),
        Text::HotkeyPause => cstr!(c"일시정지"),
        Text::HotkeyUndoAllPauses => cstr!(c"모든 일시정지 취소"),
        Text::HotkeyPreviousComparison => cstr!(c"이전 비교"),
        Text::HotkeyNextComparison => cstr!(c"다음 비교"),
        Text::HotkeyToggleTimingMethod => cstr!(c"타이밍 방법 전환"),
        Text::PropertyWidth => cstr!(c"너비"),
        Text::PropertyHeight => cstr!(c"높이"),
        Text::PropertySplits => cstr!(c"스플릿"),
        Text::PropertySplitsFilter => cstr!(c"LiveSplit 스플릿 (*.lss)"),
        Text::PropertyAutoSave => cstr!(c"리셋 시 저장"),
        Text::PropertySaveSplits => cstr!(c"스플릿 저장"),
        Text::PropertyLayout => cstr!(c"레이아웃"),
        Text::PropertyLayoutFilter => cstr!(c"LiveSplit 레이아웃 (*.lsl *.ls1l)"),
        Text::PropertyAdvancedStartGameOptions => cstr!(c"게임 시작 고급 옵션"),
        Text::PropertyGamePath => cstr!(c"게임 경로"),
        Text::PropertyGamePathFilter => cstr!(c"실행 파일 (*)"),
        Text::PropertyGameArguments => cstr!(c"게임 인수"),
        Text::PropertyWorkingDirectory => cstr!(c"작업 디렉터리"),
        Text::PropertyWorkingDirectoryFilter => cstr!(c"디렉터리"),
        Text::PropertyGameEnvironmentVars => cstr!(c"게임 환경 변수 (KEY=VALUE)"),
        Text::PropertyStartGame => cstr!(c"게임 시작"),
        Text::AutoSplitterUseLocal => cstr!(c"로컬 자동 스플리터 사용"),
        Text::AutoSplitterLocalFile => cstr!(c"로컬 자동 스플리터 파일"),
        Text::AutoSplitterLocalFileFilter => cstr!(c"LiveSplit One Auto Splitter (*.wasm)"),
        Text::AutoSplitterNoSplitsLoaded => cstr!(c"스플릿이 로드되지 않음"),
        Text::AutoSplitterActivate => cstr!(c"활성화"),
        Text::AutoSplitterDeactivate => cstr!(c"비활성화"),
        Text::AutoSplitterWebsite => cstr!(c"웹사이트"),
        Text::AutoSplitterSettingsGroup => cstr!(c"자동 스플리터 설정"),
        Text::AutoSplitterIncompatible => {
            cstr!(c"이 게임의 자동 스플리터는 LiveSplit One과 호환되지 않습니다.")
        }
        Text::AutoSplitterUnavailable => {
            cstr!(c"이 게임에 사용할 수 있는 자동 스플리터가 없습니다.")
        }
    }
}
