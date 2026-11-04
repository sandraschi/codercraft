//! CoderCraft Internationalization (i18n) Engine.
//!
//! Follows the fleet standard: typed locale enums, zero-panic string lookups,
//! multi-language translations (EN, DE, FR, ES, JA, KO, ZH),
//! covering agentic IDE studio, swarm supervisor, multi-display, and fleet depot.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AppLocale {
    #[default]
    En, // English (Global Default)
    De, // German
    Fr, // French
    Es, // Spanish
    Ja, // Japanese
    Ko, // Korean
    Zh, // Chinese
}

impl AppLocale {
    pub fn all() -> &'static [AppLocale] {
        &[
            AppLocale::En,
            AppLocale::De,
            AppLocale::Fr,
            AppLocale::Es,
            AppLocale::Ja,
            AppLocale::Ko,
            AppLocale::Zh,
        ]
    }

    pub fn code(&self) -> &'static str {
        match self {
            AppLocale::En => "en",
            AppLocale::De => "de",
            AppLocale::Fr => "fr",
            AppLocale::Es => "es",
            AppLocale::Ja => "ja",
            AppLocale::Ko => "ko",
            AppLocale::Zh => "zh",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AppLocale::En => "English (US/UK)",
            AppLocale::De => "Deutsch (DE/AT/CH)",
            AppLocale::Fr => "Français (FR)",
            AppLocale::Es => "Español (ES/LATAM)",
            AppLocale::Ja => "日本語 (JA)",
            AppLocale::Ko => "한국어 (KO)",
            AppLocale::Zh => "中文 (ZH)",
        }
    }

    pub fn flag(&self) -> &'static str {
        match self {
            AppLocale::En => "🇬🇧",
            AppLocale::De => "🇩🇪",
            AppLocale::Fr => "🇫🇷",
            AppLocale::Es => "🇪🇸",
            AppLocale::Ja => "🇯🇵",
            AppLocale::Ko => "🇰🇷",
            AppLocale::Zh => "🇨🇳",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum I18nKey {
    // Menu Bar Top Items
    File,
    Screens,
    AgentMode,
    View,
    Tools,
    Security,
    Help,

    // File Actions
    OpenWorkspace,
    SaveActiveBuffer,
    FleetProjectDepot,
    Exit,

    // Screens / Studio Modes
    IdeStudio,
    FleetDepotScreen,
    SwarmSupervisor,
    QuadDisplayConfig,

    // Agent Modes
    BuildMode,
    PlanMode,

    // Tools & Fleet Actions
    StepSwarm,
    ProbeFleetPorts,
    ScanInstructions,

    // Right Tabs & Panels
    CodeEditor,
    McpRegistry,
    LinterTerminal,
    LlmSettings,
    DiffInspector,

    // Status & Badges
    Authenticated,
    Guest,
    Tokens,
    Ready,
}

pub fn t(key: I18nKey, locale: AppLocale) -> &'static str {
    match (key, locale) {
        // Menu Bar Top Items
        (I18nKey::File, AppLocale::En) => "File",
        (I18nKey::File, AppLocale::De) => "Datei",
        (I18nKey::File, AppLocale::Fr) => "Fichier",
        (I18nKey::File, AppLocale::Es) => "Archivo",
        (I18nKey::File, AppLocale::Ja) => "ファイル",
        (I18nKey::File, AppLocale::Ko) => "파일",
        (I18nKey::File, AppLocale::Zh) => "文件",

        (I18nKey::Screens, AppLocale::En) => "Screens",
        (I18nKey::Screens, AppLocale::De) => "Bildschirme",
        (I18nKey::Screens, AppLocale::Fr) => "Écrans",
        (I18nKey::Screens, AppLocale::Es) => "Pantallas",
        (I18nKey::Screens, AppLocale::Ja) => "画面",
        (I18nKey::Screens, AppLocale::Ko) => "화면",
        (I18nKey::Screens, AppLocale::Zh) => "工作屏",

        (I18nKey::AgentMode, AppLocale::En) => "Agent Mode",
        (I18nKey::AgentMode, AppLocale::De) => "Agenten-Modus",
        (I18nKey::AgentMode, AppLocale::Fr) => "Mode Agent",
        (I18nKey::AgentMode, AppLocale::Es) => "Modo Agente",
        (I18nKey::AgentMode, AppLocale::Ja) => "エージェント・モード",
        (I18nKey::AgentMode, AppLocale::Ko) => "에이전트 모드",
        (I18nKey::AgentMode, AppLocale::Zh) => "智能体模式",

        (I18nKey::View, AppLocale::En) => "View",
        (I18nKey::View, AppLocale::De) => "Ansicht",
        (I18nKey::View, AppLocale::Fr) => "Affichage",
        (I18nKey::View, AppLocale::Es) => "Vista",
        (I18nKey::View, AppLocale::Ja) => "表示",
        (I18nKey::View, AppLocale::Ko) => "보기",
        (I18nKey::View, AppLocale::Zh) => "视图",

        (I18nKey::Tools, AppLocale::En) => "Tools",
        (I18nKey::Tools, AppLocale::De) => "Werkzeuge",
        (I18nKey::Tools, AppLocale::Fr) => "Outils",
        (I18nKey::Tools, AppLocale::Es) => "Herramientas",
        (I18nKey::Tools, AppLocale::Ja) => "ツール",
        (I18nKey::Tools, AppLocale::Ko) => "도구",
        (I18nKey::Tools, AppLocale::Zh) => "工具",

        (I18nKey::Security, AppLocale::En) => "Security",
        (I18nKey::Security, AppLocale::De) => "Sicherheit",
        (I18nKey::Security, AppLocale::Fr) => "Sécurité",
        (I18nKey::Security, AppLocale::Es) => "Seguridad",
        (I18nKey::Security, AppLocale::Ja) => "セキュリティ",
        (I18nKey::Security, AppLocale::Ko) => "보안",
        (I18nKey::Security, AppLocale::Zh) => "安全与鉴权",

        (I18nKey::Help, AppLocale::En) => "Help",
        (I18nKey::Help, AppLocale::De) => "Hilfe",
        (I18nKey::Help, AppLocale::Fr) => "Aide",
        (I18nKey::Help, AppLocale::Es) => "Ayuda",
        (I18nKey::Help, AppLocale::Ja) => "ヘルプ",
        (I18nKey::Help, AppLocale::Ko) => "도움말",
        (I18nKey::Help, AppLocale::Zh) => "帮助与文档",

        // File Actions
        (I18nKey::OpenWorkspace, AppLocale::En) => "Open Workspace...",
        (I18nKey::OpenWorkspace, AppLocale::De) => "Arbeitsbereich öffnen...",
        (I18nKey::OpenWorkspace, AppLocale::Fr) => "Ouvrir l'espace de travail...",
        (I18nKey::OpenWorkspace, AppLocale::Es) => "Abrir espacio de trabajo...",
        (I18nKey::OpenWorkspace, AppLocale::Ja) => "ワークスペースを開く...",
        (I18nKey::OpenWorkspace, AppLocale::Ko) => "작업공간 열기...",
        (I18nKey::OpenWorkspace, AppLocale::Zh) => "打开工作区...",

        (I18nKey::SaveActiveBuffer, AppLocale::En) => "Save Active Buffer (Ctrl+S)",
        (I18nKey::SaveActiveBuffer, AppLocale::De) => "Aktiven Puffer speichern (Strg+S)",
        (I18nKey::SaveActiveBuffer, AppLocale::Fr) => "Enregistrer le tampon actif (Ctrl+S)",
        (I18nKey::SaveActiveBuffer, AppLocale::Es) => "Guardar búfer activo (Ctrl+S)",
        (I18nKey::SaveActiveBuffer, AppLocale::Ja) => "現在のアクティブバッファを保存 (Ctrl+S)",
        (I18nKey::SaveActiveBuffer, AppLocale::Ko) => "현재 버퍼 저장 (Ctrl+S)",
        (I18nKey::SaveActiveBuffer, AppLocale::Zh) => "保存当前活动缓冲区 (Ctrl+S)",

        (I18nKey::FleetProjectDepot, AppLocale::En) => "Fleet Project Depot...",
        (I18nKey::FleetProjectDepot, AppLocale::De) => "Flotten-Projektdepot...",
        (I18nKey::FleetProjectDepot, AppLocale::Fr) => "Dépôt de projets de flotte...",
        (I18nKey::FleetProjectDepot, AppLocale::Es) => "Depósito de proyectos de flota...",
        (I18nKey::FleetProjectDepot, AppLocale::Ja) => "フリート・プロジェクトリポジトリ...",
        (I18nKey::FleetProjectDepot, AppLocale::Ko) => "함대 프로젝트 저장소...",
        (I18nKey::FleetProjectDepot, AppLocale::Zh) => "舰队项目总仓库...",

        (I18nKey::Exit, AppLocale::En) => "Exit",
        (I18nKey::Exit, AppLocale::De) => "Beenden",
        (I18nKey::Exit, AppLocale::Fr) => "Quitter",
        (I18nKey::Exit, AppLocale::Es) => "Salir",
        (I18nKey::Exit, AppLocale::Ja) => "終了",
        (I18nKey::Exit, AppLocale::Ko) => "종료",
        (I18nKey::Exit, AppLocale::Zh) => "退出",

        // Screens / Studio Modes
        (I18nKey::IdeStudio, AppLocale::En) => "IDE Editor Studio (Display 1)",
        (I18nKey::IdeStudio, AppLocale::De) => "IDE-Editor-Studio (Display 1)",
        (I18nKey::IdeStudio, AppLocale::Fr) => "Studio Éditeur IDE (Écran 1)",
        (I18nKey::IdeStudio, AppLocale::Es) => "Estudio de Editor IDE (Pantalla 1)",
        (I18nKey::IdeStudio, AppLocale::Ja) => "IDEエディタ・スタジオ (画面 1)",
        (I18nKey::IdeStudio, AppLocale::Ko) => "IDE 편집기 스튜디오 (화면 1)",
        (I18nKey::IdeStudio, AppLocale::Zh) => "IDE 代码编辑工作台 (主屏 1)",

        (I18nKey::FleetDepotScreen, AppLocale::En) => "Fleet Project Depot (Display 4)",
        (I18nKey::FleetDepotScreen, AppLocale::De) => "Flotten-Projektdepot (Display 4)",
        (I18nKey::FleetDepotScreen, AppLocale::Fr) => "Dépôt de projets de flotte (Écran 4)",
        (I18nKey::FleetDepotScreen, AppLocale::Es) => "Depósito de proyectos de flota (Pantalla 4)",
        (I18nKey::FleetDepotScreen, AppLocale::Ja) => "フリート・プロジェクトリポジトリ (画面 4)",
        (I18nKey::FleetDepotScreen, AppLocale::Ko) => "함대 프로젝트 저장소 (화면 4)",
        (I18nKey::FleetDepotScreen, AppLocale::Zh) => "舰队项目工程总览 (副屏 4)",

        (I18nKey::SwarmSupervisor, AppLocale::En) => "Stigmergic Swarm Supervisor (Display 3)",
        (I18nKey::SwarmSupervisor, AppLocale::De) => "Stigmergischer Schwarm-Supervisor (Display 3)",
        (I18nKey::SwarmSupervisor, AppLocale::Fr) => "Superviseur d'essaim stigmergique (Écran 3)",
        (I18nKey::SwarmSupervisor, AppLocale::Es) => "Supervisor de enjambre estigmérgico (Pantalla 3)",
        (I18nKey::SwarmSupervisor, AppLocale::Ja) => "スティグマジー群監督 (画面 3)",
        (I18nKey::SwarmSupervisor, AppLocale::Ko) => "스티그머지 에이전트 군집 감독 (화면 3)",
        (I18nKey::SwarmSupervisor, AppLocale::Zh) => "蚁群协同调度监视器 (副屏 3)",

        (I18nKey::QuadDisplayConfig, AppLocale::En) => "Multi-Display Quad Layout",
        (I18nKey::QuadDisplayConfig, AppLocale::De) => "Multi-Display-Quad-Layout",
        (I18nKey::QuadDisplayConfig, AppLocale::Fr) => "Disposition Quad multi-écrans",
        (I18nKey::QuadDisplayConfig, AppLocale::Es) => "Disposición cuádruple multipantalla",
        (I18nKey::QuadDisplayConfig, AppLocale::Ja) => "マルチディスプレイ・4画面配置",
        (I18nKey::QuadDisplayConfig, AppLocale::Ko) => "다중 디스플레이 쿼드 레이아웃",
        (I18nKey::QuadDisplayConfig, AppLocale::Zh) => "多显示器四屏全景布局",

        // Agent Modes
        (I18nKey::BuildMode, AppLocale::En) => "Build Mode (Active Editing)",
        (I18nKey::BuildMode, AppLocale::De) => "Build-Modus (Aktive Bearbeitung)",
        (I18nKey::BuildMode, AppLocale::Fr) => "Mode Construction (Édition active)",
        (I18nKey::BuildMode, AppLocale::Es) => "Modo Construcción (Edición activa)",
        (I18nKey::BuildMode, AppLocale::Ja) => "ビルドモード (コード直接編集)",
        (I18nKey::BuildMode, AppLocale::Ko) => "빌드 모드 (직접 코드 수정)",
        (I18nKey::BuildMode, AppLocale::Zh) => "构建模式 (主动代码修改)",

        (I18nKey::PlanMode, AppLocale::En) => "Plan Mode (Read-Only Roadmap)",
        (I18nKey::PlanMode, AppLocale::De) => "Plan-Modus (Schreibgeschützt)",
        (I18nKey::PlanMode, AppLocale::Fr) => "Mode Planification (Lecture seule)",
        (I18nKey::PlanMode, AppLocale::Es) => "Modo Planificación (Solo lectura)",
        (I18nKey::PlanMode, AppLocale::Ja) => "プランモード (探索と計画立案)",
        (I18nKey::PlanMode, AppLocale::Ko) => "계획 모드 (읽기 전용 로드맵)",
        (I18nKey::PlanMode, AppLocale::Zh) => "规划模式 (只读架构方案)",

        // Tools & Fleet Actions
        (I18nKey::StepSwarm, AppLocale::En) => "Step Stigmergic Swarm",
        (I18nKey::StepSwarm, AppLocale::De) => "Schwarm-Schritt ausführen",
        (I18nKey::StepSwarm, AppLocale::Fr) => "Étape d'essaim stigmergique",
        (I18nKey::StepSwarm, AppLocale::Es) => "Paso de enjambre estigmérgico",
        (I18nKey::StepSwarm, AppLocale::Ja) => "群シミュレーションを1歩進行",
        (I18nKey::StepSwarm, AppLocale::Ko) => "군집 시뮬레이션 한 단계 진행",
        (I18nKey::StepSwarm, AppLocale::Zh) => "单步推进协同蜂群演进",

        (I18nKey::ProbeFleetPorts, AppLocale::En) => "Probe Fleet TCP Ports",
        (I18nKey::ProbeFleetPorts, AppLocale::De) => "Flotten-TCP-Ports abtasten",
        (I18nKey::ProbeFleetPorts, AppLocale::Fr) => "Sonder les ports TCP de flotte",
        (I18nKey::ProbeFleetPorts, AppLocale::Es) => "Sondear puertos TCP de flota",
        (I18nKey::ProbeFleetPorts, AppLocale::Ja) => "フリートTCPポートをプローブ",
        (I18nKey::ProbeFleetPorts, AppLocale::Ko) => "함대 TCP 포트 상태 탐색",
        (I18nKey::ProbeFleetPorts, AppLocale::Zh) => "探测舰队集群TCP端口状态",

        (I18nKey::ScanInstructions, AppLocale::En) => "Scan Instructions (AGENTS.md)",
        (I18nKey::ScanInstructions, AppLocale::De) => "Instruktionen scannen (AGENTS.md)",
        (I18nKey::ScanInstructions, AppLocale::Fr) => "Analyser les instructions (AGENTS.md)",
        (I18nKey::ScanInstructions, AppLocale::Es) => "Escanear instrucciones (AGENTS.md)",
        (I18nKey::ScanInstructions, AppLocale::Ja) => "指示書スキャン (AGENTS.md)",
        (I18nKey::ScanInstructions, AppLocale::Ko) => "지침 스캔 (AGENTS.md)",
        (I18nKey::ScanInstructions, AppLocale::Zh) => "扫描工作区规范 (AGENTS.md)",

        // Right Tabs & Panels
        (I18nKey::CodeEditor, AppLocale::En) => "Code Editor",
        (I18nKey::CodeEditor, AppLocale::De) => "Code-Editor",
        (I18nKey::CodeEditor, AppLocale::Fr) => "Éditeur de code",
        (I18nKey::CodeEditor, AppLocale::Es) => "Editor de código",
        (I18nKey::CodeEditor, AppLocale::Ja) => "コードエディタ",
        (I18nKey::CodeEditor, AppLocale::Ko) => "코드 편집기",
        (I18nKey::CodeEditor, AppLocale::Zh) => "代码编辑器",

        (I18nKey::McpRegistry, AppLocale::En) => "MCP Registry",
        (I18nKey::McpRegistry, AppLocale::De) => "MCP-Register",
        (I18nKey::McpRegistry, AppLocale::Fr) => "Registre MCP",
        (I18nKey::McpRegistry, AppLocale::Es) => "Registro MCP",
        (I18nKey::McpRegistry, AppLocale::Ja) => "MCPレジストリ",
        (I18nKey::McpRegistry, AppLocale::Ko) => "MCP 레지스트리",
        (I18nKey::McpRegistry, AppLocale::Zh) => "MCP 协议注册表",

        (I18nKey::LinterTerminal, AppLocale::En) => "Linter & Terminal",
        (I18nKey::LinterTerminal, AppLocale::De) => "Linter & Terminal",
        (I18nKey::LinterTerminal, AppLocale::Fr) => "Linter et terminal",
        (I18nKey::LinterTerminal, AppLocale::Es) => "Linter y terminal",
        (I18nKey::LinterTerminal, AppLocale::Ja) => "リンター & ターミナル",
        (I18nKey::LinterTerminal, AppLocale::Ko) => "린터 및 터미널",
        (I18nKey::LinterTerminal, AppLocale::Zh) => "代码检查器与终端",

        (I18nKey::LlmSettings, AppLocale::En) => "LLM Settings",
        (I18nKey::LlmSettings, AppLocale::De) => "LLM-Einstellungen",
        (I18nKey::LlmSettings, AppLocale::Fr) => "Paramètres LLM",
        (I18nKey::LlmSettings, AppLocale::Es) => "Ajustes de LLM",
        (I18nKey::LlmSettings, AppLocale::Ja) => "LLM設定",
        (I18nKey::LlmSettings, AppLocale::Ko) => "LLM 설정",
        (I18nKey::LlmSettings, AppLocale::Zh) => "大模型配置",

        (I18nKey::DiffInspector, AppLocale::En) => "Diff Inspector",
        (I18nKey::DiffInspector, AppLocale::De) => "Diff-Inspektor",
        (I18nKey::DiffInspector, AppLocale::Fr) => "Inspecteur de diff",
        (I18nKey::DiffInspector, AppLocale::Es) => "Inspector de diferencias",
        (I18nKey::DiffInspector, AppLocale::Ja) => "差分インスペクタ",
        (I18nKey::DiffInspector, AppLocale::Ko) => "차이점 분석기",
        (I18nKey::DiffInspector, AppLocale::Zh) => "补丁对比分析器",

        // Status & Badges
        (I18nKey::Authenticated, AppLocale::En) => "Authenticated",
        (I18nKey::Authenticated, AppLocale::De) => "Authentifiziert",
        (I18nKey::Authenticated, AppLocale::Fr) => "Authentifié",
        (I18nKey::Authenticated, AppLocale::Es) => "Autenticado",
        (I18nKey::Authenticated, AppLocale::Ja) => "認証済み",
        (I18nKey::Authenticated, AppLocale::Ko) => "인증됨",
        (I18nKey::Authenticated, AppLocale::Zh) => "已通过安全认证",

        (I18nKey::Guest, AppLocale::En) => "Guest",
        (I18nKey::Guest, AppLocale::De) => "Gast",
        (I18nKey::Guest, AppLocale::Fr) => "Invité",
        (I18nKey::Guest, AppLocale::Es) => "Invitado",
        (I18nKey::Guest, AppLocale::Ja) => "ゲスト",
        (I18nKey::Guest, AppLocale::Ko) => "게스트",
        (I18nKey::Guest, AppLocale::Zh) => "访客模式",

        (I18nKey::Tokens, AppLocale::En) => "Tokens",
        (I18nKey::Tokens, AppLocale::De) => "Tokens",
        (I18nKey::Tokens, AppLocale::Fr) => "Jetons",
        (I18nKey::Tokens, AppLocale::Es) => "Tokens",
        (I18nKey::Tokens, AppLocale::Ja) => "トークン",
        (I18nKey::Tokens, AppLocale::Ko) => "토큰",
        (I18nKey::Tokens, AppLocale::Zh) => "Token消耗",

        (I18nKey::Ready, AppLocale::En) => "CoderCraft Sovereign Kernel Ready",
        (I18nKey::Ready, AppLocale::De) => "CoderCraft Sovereign Kernel bereit",
        (I18nKey::Ready, AppLocale::Fr) => "Noyau souverain CoderCraft prêt",
        (I18nKey::Ready, AppLocale::Es) => "Núcleo soberano CoderCraft listo",
        (I18nKey::Ready, AppLocale::Ja) => "CoderCraft ソブリンコア 準備完了",
        (I18nKey::Ready, AppLocale::Ko) => "CoderCraft 주권 커널 준비완료",
        (I18nKey::Ready, AppLocale::Zh) => "CoderCraft 自主代码中枢已就绪",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codercraft_i18n_locales() {
        assert_eq!(AppLocale::all().len(), 7);
        for loc in AppLocale::all() {
            assert!(!loc.code().is_empty());
            assert!(!loc.display_name().is_empty());
            assert!(!loc.flag().is_empty());
            assert!(!t(I18nKey::File, *loc).is_empty());
            assert!(!t(I18nKey::Screens, *loc).is_empty());
            assert!(!t(I18nKey::AgentMode, *loc).is_empty());
            assert!(!t(I18nKey::View, *loc).is_empty());
            assert!(!t(I18nKey::Tools, *loc).is_empty());
            assert!(!t(I18nKey::Security, *loc).is_empty());
            assert!(!t(I18nKey::Help, *loc).is_empty());
            assert!(!t(I18nKey::IdeStudio, *loc).is_empty());
            assert!(!t(I18nKey::Ready, *loc).is_empty());
        }
    }
}
