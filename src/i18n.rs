//! 国际化（i18n）支持模块。
//!
//! 设计要点：
//! - 零外部依赖，单文件实现。
//! - [`Language`] 枚举承载支持的语言；[`detect`] 通过 `LC_ALL`/`LANG` 环境变量探测。
//! - 全局语言状态用 [`std::sync::OnceLock`] 惰性初始化，初始化失败不会 panic，仅回退到英文。
//! - 内部 [`translate`] 是纯函数（语言 + key -> 静态字符串），单测直接覆盖它；
//!   对外暴露 [`tr`] 读取全局语言后委托给 [`translate`]。

use std::sync::OnceLock;

/// herdr UI 支持的语言。
///
/// 默认值为 [`Language::En`]（英文），作为探测失败时的兜底。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Language {
    #[default]
    En,
    ZhCn,
}

impl Language {
    /// 将语言代码字符串解析为 [`Language`]，大小写不敏感。
    ///
    /// 支持的代码：
    /// - 英文：`en`、`en-us`、`en-gb`
    /// - 简体中文：`zh`、`zh-cn`、`zh_cn`、`zh-hans`、`zh_hans_cn`
    ///
    /// 注意：POSIX `C`/`POSIX` 不是自然语言，这里不识别（探测阶段单独处理）。
    /// 未知代码返回 `None`。
    pub fn from_code(code: &str) -> Option<Self> {
        // 归一化：去修饰后缀（@modifier）、转小写、把下划线统一为连字符。
        let normalized = code
            .split('@')
            .next()
            .unwrap_or("")
            .to_lowercase()
            .replace('_', "-");
        if matches!(
            normalized.as_str(),
            "en" | "en-us" | "en-gb" | "en-au" | "en-ca"
        ) {
            return Some(Language::En);
        }
        // 任意 zh 开头的代码（含繁体 zh-tw/zh-hant）均视作中文 UI；
        // herdr 目前只提供简体翻译，繁体用户会拿到简体文案，仍优于英文。
        if normalized == "zh" || normalized.starts_with("zh-") {
            return Some(Language::ZhCn);
        }
        None
    }
}

/// 探测当前应使用的语言。
///
/// 优先级：`LC_ALL` -> `LANG`；任一命中 `zh*` 即判定为中文，否则回退英文。
/// 探测仅在初始化时执行一次，结果缓存进全局 [`OnceLock`]。
fn detect() -> Language {
    // 依次检查 LC_ALL、LANG；取首个非空白值判断语言。
    // 空字符串（LC_ALL=""）视为未设置，跳过继续查下一个变量。
    for var in ["LC_ALL", "LANG"] {
        if let Some(lang) = std::env::var(var)
            .ok()
            .filter(|raw| !raw.trim().is_empty())
            .as_deref()
            .map(language_from_locale)
        {
            return lang;
        }
    }
    Language::En
}

/// 把一个 locale 字符串（如 `zh_CN.UTF-8`、`en_US`、`C`）归约为 [`Language`]。
///
/// 规则：`C`/`POSIX` 一律英文；否则取主语言段（`.`/`@` 前的部分），
/// 起始为 `zh` 即中文，其余走 [`Language::from_code`] 兜底。
fn language_from_locale(locale: &str) -> Language {
    // 去掉编码后缀（.UTF-8）与修饰符（@modifier）。
    let primary = locale.split(['.', '@']).next().unwrap_or(locale).trim();
    if primary.is_empty() {
        return Language::En;
    }
    // POSIX 的 C/POSIX locale 强制英文。
    if primary.eq_ignore_ascii_case("C") || primary.eq_ignore_ascii_case("POSIX") {
        return Language::En;
    }
    // 取语言主语（首个 _ 或 - 之前）。
    let tag = primary.split(['_', '-']).next().unwrap_or(primary);
    if tag.eq_ignore_ascii_case("zh") {
        return Language::ZhCn;
    }
    Language::from_code(primary).unwrap_or(Language::En)
}

/// 全部可翻译的文案 key。
///
/// 每个变体对应 UI 中一处硬编码英文字符串。新增 UI 文案时：
/// 1. 在此枚举追加变体；
/// 2. 在 [`translate`] 的两张 match 表里各补一条；
/// 3. 在 `every_key_has_both_translations` 测试的 `all_keys` 数组里补上。
///
/// match 的穷尽性由编译器保证（漏写分支会编译失败），测试则兜底校验非空。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TranslationKey {
    // ── keybind_help.rs: 分组标题 ──
    GlobalGroup,
    NavigationGroup,
    WorkspacesTabsGroup,
    PanesGroup,
    CustomGroup,

    // ── keybind_help.rs: global 分组文案 ──
    PrefixMode,
    KeybindsLabel,
    Settings,
    Detach,
    ReloadConfig,
    OpenNotificationTarget,

    // ── keybind_help.rs: navigation 分组文案 ──
    Back,
    WorkspaceList,
    MoveFocus,
    CyclePane,
    OpenWorkspace,
    SwitchWorkspace,

    // ── keybind_help.rs: workspaces / tabs 分组文案 ──
    WorkspaceNavigation,
    SessionNavigator,
    NewWorkspace,
    NewWorktree,
    OpenWorktree,
    DeleteWorktreeCheckout,
    RenameWorkspace,
    CloseWorkspace,
    PreviousWorkspace,
    NextWorkspace,
    SwitchWorkspace19,
    PreviousAgent,
    NextAgent,
    FocusAgent19,
    NewTab,
    RenameTab,
    PreviousTab,
    NextTab,
    SwitchTab19,
    CloseTab,

    // ── keybind_help.rs: panes 分组文案 ──
    SplitVertical,
    SplitHorizontal,
    ClosePane,
    RenamePane,
    EditScrollback,
    CopyMode,
    ZoomPane,
    ResizeMode,
    ToggleSidebar,
    FocusPaneLeft,
    FocusPaneDown,
    FocusPaneUp,
    FocusPaneRight,
    CyclePaneNext,
    CyclePanePrevious,
    LastPane,

    // ── keybind_help.rs: custom 分组文案 ──
    CustomCommand,

    // ── keybind_help.rs: 其余界面文案 ──
    Unset,
    NoMatchingKeybinds,
    Close,
    PressSlashToFilter,
    FilterLabel,
    ClearLabel,
    ScrollLabel,
    SearchLabel,

    // ── menus.rs: 模式徽章 ──
    ModePrefix,
    ModeCopy,
    ModeNavigate,
    ModeResize,

    // ── menus.rs: PREFIX overlay 文案 ──
    Cancel,
    SendPrefix,
    WorkspaceNavShort,

    // ── menus.rs: COPY overlay 文案 ──
    EnterSearchEscCancel,
    Selecting,
    Select,
    Exit,
    ClearQExit,
    Move,
    Repeat,
    Copy,

    // ── menus.rs: NAVIGATE overlay 文案 ──
    WsShort,
    Pane,
    NavigatorShort,
    SplitVertSymbol,
    SplitHorizSymbol,
    Zoom,
    ResizeShort,
    UpdateReady,

    // ── menus.rs: RESIZE overlay 文案 ──
    Width,
    Height,
    Done,

    // ── global menu 文案 ──
    WhatsNew,

    // ── dialogs.rs: 通用按钮 ──
    Save,
    Confirm,
    Open,
    CreateAndOpen,
    DeleteAnyway,
    Remove,

    // ── dialogs.rs: new worktree 对话框 ──
    Branch,
    Checkout,
    Creating,

    // ── dialogs.rs: remove worktree 对话框 ──
    DeleteWorktreeCheckoutQ,
    RemovesCheckoutFolder,
    BranchNotDeletedWorkspaceWillClose,
    DirtyWillBeDeleted,
    Removing,

    // ── dialogs.rs: open worktree 对话框 ──
    NoMatchingWorktrees,
    FilterWorktrees,
    Checkouts,

    // ── dialogs.rs: confirm close 对话框 ──
    CloseWorkspaceQ,
    CloseWorktreeGroupQ,
    PaneUnitSingular,
    PaneUnitPlural,
    WorkspaceUnitSingular,
    WorkspaceUnitPlural,

    // ── context menu 文案 ──
    CmRename,
    CmClose,
    CmCloseGroup,
    CmNewWorktree,
    CmOpenWorktree,
    CmDeleteWorktree,
    CmCollapse,
    CmExpand,
    CmNewTab,
    CmRenamePane,
    CmClearPaneName,
    CmSwapWithFocusedPane,
    CmSplitRight,
    CmSplitDown,
    CmZoom,
    CmClosePane,

    // ── mobile.rs 文案 ──
    MobileSwitch,
    NoWorkspace,
    Filtered,
    Agents,
    NoMatchingAgents,
    Spaces,
    NewWorkspaceMobile,
    Tabs,
    NewTabMobile,
    TabPrefix,
    Menu,
    NoAgents,
    AllIdle,
    BlockedLabel,
    DoneLabel,
    WorkingLabel,
    IdleLabel,
    WaitingAgent,
    DoneAgent,

    // ── sidebar.rs 文案 ──
    SidebarNew,
    SortGrouped,
    SortPriority,
    UnknownLabel,
    CopiedToClipboard,

    // ── settings.rs 文案 ──
    SoundAlerts,
    SoundAlertsDesc,
    NotificationPopups,
    NotificationPopupsDesc,
    ToastInsideHerdr,
    ToastViaTerminal,
    ToastViaSystem,
    ToggleOn,
    ToggleOff,
    AgentBorderLabels,
    AgentBorderLabelsDesc,
    AgentIntegrations,
    AgentIntegrationsDesc,
    NoIntegrationTargets,
    IntegrationsHintInstall,
    IntegrationsAllInstalled,
    IntegrationsNoneFound,
    Install,
    Apply,
    SectionHint,

    // ── settings.rs 分区页签 ──
    SectionTheme,
    SectionSound,
    SectionToast,
    SectionPaneLabels,
    SectionIntegrations,

    // ── integration 状态文案 ──
    IntegrationInstalled,
    IntegrationUpdateAvailable,
    IntegrationAvailable,
    IntegrationNotFound,
    IntegrationsAllCurrent,
}

/// 纯函数：按语言与 key 查找翻译字符串。
///
/// 返回的 `&'static str` 在整个程序生命周期内有效，便于直接喂给 UI 组件。
/// 单测应直接覆盖此函数，而非依赖全局状态。两张 match 表必须穷尽所有变体。
pub(crate) fn translate(lang: Language, key: TranslationKey) -> &'static str {
    match lang {
        Language::En => match key {
            TranslationKey::GlobalGroup => "global",
            TranslationKey::NavigationGroup => "navigation",
            TranslationKey::WorkspacesTabsGroup => "workspaces / tabs",
            TranslationKey::PanesGroup => "panes",
            TranslationKey::CustomGroup => "custom",

            TranslationKey::PrefixMode => "prefix mode",
            TranslationKey::KeybindsLabel => "keybinds",
            TranslationKey::Settings => "settings",
            TranslationKey::Detach => "detach",
            TranslationKey::ReloadConfig => "reload config",
            TranslationKey::OpenNotificationTarget => "open notification target",

            TranslationKey::Back => "back",
            TranslationKey::WorkspaceList => "workspace list",
            TranslationKey::MoveFocus => "move focus",
            TranslationKey::CyclePane => "cycle pane",
            TranslationKey::OpenWorkspace => "open workspace",
            TranslationKey::SwitchWorkspace => "switch workspace",

            TranslationKey::WorkspaceNavigation => "workspace navigation",
            TranslationKey::SessionNavigator => "session navigator",
            TranslationKey::NewWorkspace => "new workspace",
            TranslationKey::NewWorktree => "new worktree",
            TranslationKey::OpenWorktree => "open worktree",
            TranslationKey::DeleteWorktreeCheckout => "delete worktree checkout",
            TranslationKey::RenameWorkspace => "rename workspace",
            TranslationKey::CloseWorkspace => "close workspace",
            TranslationKey::PreviousWorkspace => "previous workspace",
            TranslationKey::NextWorkspace => "next workspace",
            TranslationKey::SwitchWorkspace19 => "switch workspace 1-9",
            TranslationKey::PreviousAgent => "previous agent",
            TranslationKey::NextAgent => "next agent",
            TranslationKey::FocusAgent19 => "focus agent 1-9",
            TranslationKey::NewTab => "new tab",
            TranslationKey::RenameTab => "rename tab",
            TranslationKey::PreviousTab => "previous tab",
            TranslationKey::NextTab => "next tab",
            TranslationKey::SwitchTab19 => "switch tab 1-9",
            TranslationKey::CloseTab => "close tab",

            TranslationKey::SplitVertical => "split vertical",
            TranslationKey::SplitHorizontal => "split horizontal",
            TranslationKey::ClosePane => "close pane",
            TranslationKey::RenamePane => "rename pane",
            TranslationKey::EditScrollback => "edit scrollback",
            TranslationKey::CopyMode => "copy mode",
            TranslationKey::ZoomPane => "zoom pane",
            TranslationKey::ResizeMode => "resize mode",
            TranslationKey::ToggleSidebar => "toggle sidebar",
            TranslationKey::FocusPaneLeft => "focus pane left",
            TranslationKey::FocusPaneDown => "focus pane down",
            TranslationKey::FocusPaneUp => "focus pane up",
            TranslationKey::FocusPaneRight => "focus pane right",
            TranslationKey::CyclePaneNext => "cycle pane next",
            TranslationKey::CyclePanePrevious => "cycle pane previous",
            TranslationKey::LastPane => "last pane",

            TranslationKey::CustomCommand => "custom command",

            TranslationKey::Unset => "unset",
            TranslationKey::NoMatchingKeybinds => "no matching keybinds",
            TranslationKey::Close => "close",
            TranslationKey::PressSlashToFilter => "press / to filter by command or shortcut",
            TranslationKey::FilterLabel => "filter",
            TranslationKey::ClearLabel => "clear",
            TranslationKey::ScrollLabel => "scroll",
            TranslationKey::SearchLabel => "search",

            TranslationKey::ModePrefix => "PREFIX",
            TranslationKey::ModeCopy => "COPY",
            TranslationKey::ModeNavigate => "NAVIGATE",
            TranslationKey::ModeResize => "RESIZE",

            TranslationKey::Cancel => "cancel",
            TranslationKey::SendPrefix => "send prefix",
            TranslationKey::WorkspaceNavShort => "workspace nav",

            TranslationKey::EnterSearchEscCancel => "enter search  esc cancel",
            TranslationKey::Selecting => "selecting",
            TranslationKey::Select => "select",
            TranslationKey::Exit => "exit",
            TranslationKey::ClearQExit => "clear  q exit",
            TranslationKey::Move => "move",
            TranslationKey::Repeat => "repeat",
            TranslationKey::Copy => "copy",

            TranslationKey::WsShort => "ws",
            TranslationKey::Pane => "pane",
            TranslationKey::NavigatorShort => "navigator",
            TranslationKey::SplitVertSymbol => "split│",
            TranslationKey::SplitHorizSymbol => "split─",
            TranslationKey::Zoom => "zoom",
            TranslationKey::ResizeShort => "resize",
            TranslationKey::UpdateReady => "update ready",

            TranslationKey::Width => "width",
            TranslationKey::Height => "height",
            TranslationKey::Done => "done",

            TranslationKey::WhatsNew => "what's new",

            TranslationKey::Save => "save",
            TranslationKey::Confirm => "confirm",
            TranslationKey::Open => "open",
            TranslationKey::CreateAndOpen => "create and open",
            TranslationKey::DeleteAnyway => "delete anyway",
            TranslationKey::Remove => "remove",

            TranslationKey::Branch => "branch",
            TranslationKey::Checkout => "checkout",
            TranslationKey::Creating => "creating…",

            TranslationKey::DeleteWorktreeCheckoutQ => "delete worktree checkout?",
            TranslationKey::RemovesCheckoutFolder => "This removes the checkout folder:",
            TranslationKey::BranchNotDeletedWorkspaceWillClose => {
                "The branch is not deleted. The Herdr workspace will close."
            }
            TranslationKey::DirtyWillBeDeleted => {
                "Dirty or untracked files will be permanently deleted."
            }
            TranslationKey::Removing => "removing…",

            TranslationKey::NoMatchingWorktrees => "no matching worktrees",
            TranslationKey::FilterWorktrees => "filter worktrees",
            TranslationKey::Checkouts => "checkouts",

            TranslationKey::CloseWorkspaceQ => "Close workspace?",
            TranslationKey::CloseWorktreeGroupQ => "Close worktree group?",
            TranslationKey::PaneUnitSingular => "pane",
            TranslationKey::PaneUnitPlural => "panes",
            TranslationKey::WorkspaceUnitSingular => "workspace",
            TranslationKey::WorkspaceUnitPlural => "workspaces",

            TranslationKey::CmRename => "Rename",
            TranslationKey::CmClose => "Close",
            TranslationKey::CmCloseGroup => "Close group",
            TranslationKey::CmNewWorktree => "New worktree",
            TranslationKey::CmOpenWorktree => "Open worktree...",
            TranslationKey::CmDeleteWorktree => "Delete worktree checkout...",
            TranslationKey::CmCollapse => "Collapse",
            TranslationKey::CmExpand => "Expand",
            TranslationKey::CmNewTab => "New tab",
            TranslationKey::CmRenamePane => "Rename pane",
            TranslationKey::CmClearPaneName => "Clear pane name",
            TranslationKey::CmSwapWithFocusedPane => "Swap with focused pane",
            TranslationKey::CmSplitRight => "Split right",
            TranslationKey::CmSplitDown => "Split down",
            TranslationKey::CmZoom => "Zoom",
            TranslationKey::CmClosePane => "Close pane",

            TranslationKey::MobileSwitch => "switch",
            TranslationKey::NoWorkspace => "no workspace",
            TranslationKey::Filtered => "filtered",
            TranslationKey::Agents => "agents",
            TranslationKey::NoMatchingAgents => "no matching agents",
            TranslationKey::Spaces => "spaces",
            TranslationKey::NewWorkspaceMobile => "+ new workspace",
            TranslationKey::Tabs => "tabs",
            TranslationKey::NewTabMobile => "+ new tab",
            TranslationKey::TabPrefix => "tab",
            TranslationKey::Menu => "menu",
            TranslationKey::NoAgents => "no agents",
            TranslationKey::AllIdle => "all idle",
            TranslationKey::BlockedLabel => "blocked",
            TranslationKey::DoneLabel => "done",
            TranslationKey::WorkingLabel => "working",
            TranslationKey::IdleLabel => "idle",
            TranslationKey::WaitingAgent => "waiting",
            TranslationKey::DoneAgent => "done",

            TranslationKey::SidebarNew => "new",

            TranslationKey::SoundAlerts => "sound alerts",
            TranslationKey::SoundAlertsDesc => "play sounds when agents change state in background",
            TranslationKey::NotificationPopups => "notification popups",
            TranslationKey::NotificationPopupsDesc => {
                "choose where background popup notifications should appear"
            }
            TranslationKey::ToastInsideHerdr => "inside herdr",
            TranslationKey::ToastViaTerminal => "via terminal",
            TranslationKey::ToastViaSystem => "via system",
            TranslationKey::ToggleOn => "on",
            TranslationKey::ToggleOff => "off",
            TranslationKey::AgentBorderLabels => "agent border labels",
            TranslationKey::AgentBorderLabelsDesc => {
                "show detected agent names in split pane borders"
            }
            TranslationKey::AgentIntegrations => "agent integrations",
            TranslationKey::AgentIntegrationsDesc => {
                "let agents report state directly instead of relying only on process detection"
            }
            TranslationKey::NoIntegrationTargets => "no integration targets available",
            TranslationKey::IntegrationsHintInstall => {
                "press install to add available or outdated integrations"
            }
            TranslationKey::IntegrationsAllInstalled => "all detected integrations are installed",
            TranslationKey::IntegrationsNoneFound => "no supported agent CLIs found on PATH",
            TranslationKey::Install => "install",
            TranslationKey::Apply => "apply",
            TranslationKey::SectionHint => "section",

            TranslationKey::SectionTheme => "theme",
            TranslationKey::SectionSound => "sound",
            TranslationKey::SectionToast => "toasts",
            TranslationKey::SectionPaneLabels => "pane labels",
            TranslationKey::SectionIntegrations => "integrations",

            TranslationKey::IntegrationInstalled => "installed",
            TranslationKey::IntegrationUpdateAvailable => "update available",
            TranslationKey::IntegrationAvailable => "available",
            TranslationKey::IntegrationNotFound => "not found",
            TranslationKey::IntegrationsAllCurrent => "all detected integrations are current",

            TranslationKey::SortGrouped => "grouped",
            TranslationKey::SortPriority => "priority",
            TranslationKey::UnknownLabel => "unknown",
            TranslationKey::CopiedToClipboard => "copied to clipboard",
        },
        Language::ZhCn => match key {
            TranslationKey::GlobalGroup => "全局",
            TranslationKey::NavigationGroup => "导航",
            TranslationKey::WorkspacesTabsGroup => "工作区与标签页",
            TranslationKey::PanesGroup => "窗格",
            TranslationKey::CustomGroup => "自定义",

            TranslationKey::PrefixMode => "前缀模式",
            TranslationKey::KeybindsLabel => "快捷键",
            TranslationKey::Settings => "设置",
            TranslationKey::Detach => "分离",
            TranslationKey::ReloadConfig => "重新加载配置",
            TranslationKey::OpenNotificationTarget => "打开通知目标",

            TranslationKey::Back => "返回",
            TranslationKey::WorkspaceList => "工作区列表",
            TranslationKey::MoveFocus => "移动焦点",
            TranslationKey::CyclePane => "循环切换窗格",
            TranslationKey::OpenWorkspace => "打开工作区",
            TranslationKey::SwitchWorkspace => "切换工作区",

            TranslationKey::WorkspaceNavigation => "工作区导航",
            TranslationKey::SessionNavigator => "会话导航",
            TranslationKey::NewWorkspace => "新建工作区",
            TranslationKey::NewWorktree => "新建 worktree",
            TranslationKey::OpenWorktree => "打开 worktree",
            TranslationKey::DeleteWorktreeCheckout => "删除 worktree 检出",
            TranslationKey::RenameWorkspace => "重命名工作区",
            TranslationKey::CloseWorkspace => "关闭工作区",
            TranslationKey::PreviousWorkspace => "上一个工作区",
            TranslationKey::NextWorkspace => "下一个工作区",
            TranslationKey::SwitchWorkspace19 => "切换工作区 1-9",
            TranslationKey::PreviousAgent => "上一个 agent",
            TranslationKey::NextAgent => "下一个 agent",
            TranslationKey::FocusAgent19 => "聚焦 agent 1-9",
            TranslationKey::NewTab => "新建标签页",
            TranslationKey::RenameTab => "重命名标签页",
            TranslationKey::PreviousTab => "上一个标签页",
            TranslationKey::NextTab => "下一个标签页",
            TranslationKey::SwitchTab19 => "切换标签页 1-9",
            TranslationKey::CloseTab => "关闭标签页",

            TranslationKey::SplitVertical => "垂直分割",
            TranslationKey::SplitHorizontal => "水平分割",
            TranslationKey::ClosePane => "关闭窗格",
            TranslationKey::RenamePane => "重命名窗格",
            TranslationKey::EditScrollback => "编辑回滚",
            TranslationKey::CopyMode => "复制模式",
            TranslationKey::ZoomPane => "缩放窗格",
            TranslationKey::ResizeMode => "调整大小模式",
            TranslationKey::ToggleSidebar => "切换侧边栏",
            TranslationKey::FocusPaneLeft => "左移焦点",
            TranslationKey::FocusPaneDown => "下移焦点",
            TranslationKey::FocusPaneUp => "上移焦点",
            TranslationKey::FocusPaneRight => "右移焦点",
            TranslationKey::CyclePaneNext => "下一个窗格",
            TranslationKey::CyclePanePrevious => "上一个窗格",
            TranslationKey::LastPane => "最后一个窗格",

            TranslationKey::CustomCommand => "自定义命令",

            TranslationKey::Unset => "未设置",
            TranslationKey::NoMatchingKeybinds => "无匹配的快捷键",
            TranslationKey::Close => "关闭",
            TranslationKey::PressSlashToFilter => "按 / 按命令或快捷键筛选",
            TranslationKey::FilterLabel => "筛选",
            TranslationKey::ClearLabel => "清除",
            TranslationKey::ScrollLabel => "滚动",
            TranslationKey::SearchLabel => "搜索",

            TranslationKey::ModePrefix => "前缀",
            TranslationKey::ModeCopy => "复制",
            TranslationKey::ModeNavigate => "导航",
            TranslationKey::ModeResize => "调整",

            TranslationKey::Cancel => "取消",
            TranslationKey::SendPrefix => "发送前缀",
            TranslationKey::WorkspaceNavShort => "工作区导航",

            TranslationKey::EnterSearchEscCancel => "enter 搜索  esc 取消",
            TranslationKey::Selecting => "选择中",
            TranslationKey::Select => "选择",
            TranslationKey::Exit => "退出",
            TranslationKey::ClearQExit => "清除  q 退出",
            TranslationKey::Move => "移动",
            TranslationKey::Repeat => "重复",
            TranslationKey::Copy => "复制",

            TranslationKey::WsShort => "工作区",
            TranslationKey::Pane => "窗格",
            TranslationKey::NavigatorShort => "导航器",
            TranslationKey::SplitVertSymbol => "分割│",
            TranslationKey::SplitHorizSymbol => "分割─",
            TranslationKey::Zoom => "缩放",
            TranslationKey::ResizeShort => "调整",
            TranslationKey::UpdateReady => "有更新",

            TranslationKey::Width => "宽度",
            TranslationKey::Height => "高度",
            TranslationKey::Done => "完成",

            TranslationKey::WhatsNew => "新功能",

            TranslationKey::Save => "保存",
            TranslationKey::Confirm => "确认",
            TranslationKey::Open => "打开",
            TranslationKey::CreateAndOpen => "创建并打开",
            TranslationKey::DeleteAnyway => "仍然删除",
            TranslationKey::Remove => "移除",

            TranslationKey::Branch => "分支",
            TranslationKey::Checkout => "检出路径",
            TranslationKey::Creating => "创建中…",

            TranslationKey::DeleteWorktreeCheckoutQ => "删除 worktree 检出？",
            TranslationKey::RemovesCheckoutFolder => "这将移除检出文件夹：",
            TranslationKey::BranchNotDeletedWorkspaceWillClose => {
                "分支不会被删除。Herdr 工作区将关闭。"
            }
            TranslationKey::DirtyWillBeDeleted => "脏文件或未跟踪文件将被永久删除。",
            TranslationKey::Removing => "移除中…",

            TranslationKey::NoMatchingWorktrees => "无匹配的 worktree",
            TranslationKey::FilterWorktrees => "筛选 worktree",
            TranslationKey::Checkouts => "个检出",

            TranslationKey::CloseWorkspaceQ => "关闭工作区？",
            TranslationKey::CloseWorktreeGroupQ => "关闭 worktree 组？",
            TranslationKey::PaneUnitSingular => "个窗格",
            TranslationKey::PaneUnitPlural => "个窗格",
            TranslationKey::WorkspaceUnitSingular => "个工作区",
            TranslationKey::WorkspaceUnitPlural => "个工作区",

            TranslationKey::CmRename => "重命名",
            TranslationKey::CmClose => "关闭",
            TranslationKey::CmCloseGroup => "关闭组",
            TranslationKey::CmNewWorktree => "新建 worktree",
            TranslationKey::CmOpenWorktree => "打开 worktree...",
            TranslationKey::CmDeleteWorktree => "删除 worktree 检出...",
            TranslationKey::CmCollapse => "折叠",
            TranslationKey::CmExpand => "展开",
            TranslationKey::CmNewTab => "新建标签页",
            TranslationKey::CmRenamePane => "重命名窗格",
            TranslationKey::CmClearPaneName => "清除窗格名",
            TranslationKey::CmSwapWithFocusedPane => "与聚焦窗格交换",
            TranslationKey::CmSplitRight => "向右分割",
            TranslationKey::CmSplitDown => "向下分割",
            TranslationKey::CmZoom => "缩放",
            TranslationKey::CmClosePane => "关闭窗格",

            TranslationKey::MobileSwitch => "切换",
            TranslationKey::NoWorkspace => "无工作区",
            TranslationKey::Filtered => "已筛选",
            TranslationKey::Agents => "agent",
            TranslationKey::NoMatchingAgents => "无匹配的 agent",
            TranslationKey::Spaces => "工作区",
            TranslationKey::NewWorkspaceMobile => "+ 新建工作区",
            TranslationKey::Tabs => "标签页",
            TranslationKey::NewTabMobile => "+ 新建标签页",
            TranslationKey::TabPrefix => "标签页",
            TranslationKey::Menu => "菜单",
            TranslationKey::NoAgents => "无 agent",
            TranslationKey::AllIdle => "全部空闲",
            TranslationKey::BlockedLabel => "阻塞",
            TranslationKey::DoneLabel => "完成",
            TranslationKey::WorkingLabel => "工作中",
            TranslationKey::IdleLabel => "空闲",
            TranslationKey::WaitingAgent => "等待中",
            TranslationKey::DoneAgent => "已完成",

            TranslationKey::SidebarNew => "新建",

            TranslationKey::SoundAlerts => "声音提醒",
            TranslationKey::SoundAlertsDesc => "后台 agent 状态变化时播放提示音",
            TranslationKey::NotificationPopups => "通知弹窗",
            TranslationKey::NotificationPopupsDesc => "选择后台通知弹窗的显示位置",
            TranslationKey::ToastInsideHerdr => "herdr 内",
            TranslationKey::ToastViaTerminal => "经终端",
            TranslationKey::ToastViaSystem => "经系统",
            TranslationKey::ToggleOn => "开启",
            TranslationKey::ToggleOff => "关闭",
            TranslationKey::AgentBorderLabels => "agent 边框标签",
            TranslationKey::AgentBorderLabelsDesc => "在分割窗格边框中显示检测到的 agent 名称",
            TranslationKey::AgentIntegrations => "agent 集成",
            TranslationKey::AgentIntegrationsDesc => "让 agent 直接上报状态，而非仅依赖进程检测",
            TranslationKey::NoIntegrationTargets => "无可用的集成目标",
            TranslationKey::IntegrationsHintInstall => "按「安装」添加可用或过时的集成",
            TranslationKey::IntegrationsAllInstalled => "所有检测到的集成均已安装",
            TranslationKey::IntegrationsNoneFound => "PATH 中未找到受支持的 agent CLI",
            TranslationKey::Install => "安装",
            TranslationKey::Apply => "应用",
            TranslationKey::SectionHint => "切换分区",

            TranslationKey::SectionTheme => "主题",
            TranslationKey::SectionSound => "声音",
            TranslationKey::SectionToast => "通知",
            TranslationKey::SectionPaneLabels => "窗格标签",
            TranslationKey::SectionIntegrations => "集成",

            TranslationKey::IntegrationInstalled => "已安装",
            TranslationKey::IntegrationUpdateAvailable => "有更新",
            TranslationKey::IntegrationAvailable => "可安装",
            TranslationKey::IntegrationNotFound => "未找到",
            TranslationKey::IntegrationsAllCurrent => "所有检测到的集成均为最新",

            TranslationKey::SortGrouped => "分组",
            TranslationKey::SortPriority => "优先级",
            TranslationKey::UnknownLabel => "未知",
            TranslationKey::CopiedToClipboard => "已复制到剪贴板",
        },
    }
}

/// 全局语言状态，惰性初始化，永不 panic。
static LANGUAGE: OnceLock<Language> = OnceLock::new();

/// 读取当前生效的语言；若尚未初始化则探测并缓存。
///
/// 绝不 panic：探测失败时回退到 [`Language::En`]。
pub(crate) fn current_language() -> Language {
    *LANGUAGE.get_or_init(detect)
}

/// 对外翻译入口：读取全局语言后委托给 [`translate`]。
///
/// 调用方可放心使用，无需关心初始化时机。
pub fn tr(key: TranslationKey) -> &'static str {
    translate(current_language(), key)
}

/// 显式初始化 i18n 模块。
///
/// 在 `main` 早期（参数收集后）调用一次即可；后续 [`tr`] 也会兜底初始化。
pub fn init() {
    current_language();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // 环境变量是进程级共享状态，并行的 env 测试会互相污染，故用该锁串行化所有
    // 读取/写入 LC_ALL、LANG 的测试。
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// 在作用域内临时设置环境变量，作用域结束自动恢复（含未设置状态）。
    struct EnvGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            // 持有 ENV_LOCK 保证无并行竞争；Guard 退出时恢复原值。
            let original = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, original }
        }

        fn unset(key: &'static str) -> Self {
            let original = std::env::var(key).ok();
            std::env::remove_var(key);
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    // ── from_code 各分支 ──

    #[test]
    fn from_code_english_variants() {
        for code in [
            "en", "EN", "En", "en-US", "en-us", "en_US", "en-GB", "en-AU",
        ] {
            assert_eq!(Language::from_code(code), Some(Language::En), "code={code}");
        }
    }

    #[test]
    fn from_code_chinese_variants() {
        for code in [
            "zh",
            "ZH",
            "zh-CN",
            "zh-cn",
            "zh_CN",
            "zh-Hans",
            "zh_Hans",
            "zh-Hans-CN",
            "zh_hans_cn",
            "zh-TW",
            "zh-Hant",
        ] {
            assert_eq!(
                Language::from_code(code),
                Some(Language::ZhCn),
                "code={code}"
            );
        }
    }

    #[test]
    fn from_code_strips_modifier() {
        // e.g. zh_CN.UTF-8@pinyin 中的 @pinyin 应被剥离。
        assert_eq!(
            Language::from_code("zh_CN.UTF-8@pinyin"),
            Some(Language::ZhCn)
        );
        assert_eq!(Language::from_code("en_US@latin"), Some(Language::En));
    }

    #[test]
    fn from_code_unknown_returns_none() {
        for code in ["fr", "ja", "de", "", "xx", "c", "posix"] {
            assert_eq!(Language::from_code(code), None, "code={code}");
        }
    }

    // ── detect() 各场景 ──

    #[test]
    fn detect_lc_all_preferred_over_lang() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _lcall = EnvGuard::set("LC_ALL", "zh_CN.UTF-8");
        let _lang = EnvGuard::set("LANG", "en_US.UTF-8");
        assert_eq!(detect(), Language::ZhCn);
    }

    #[test]
    fn detect_falls_back_to_lang_when_no_lc_all() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _lcall = EnvGuard::unset("LC_ALL");
        let _lang = EnvGuard::set("LANG", "zh_CN.UTF-8");
        assert_eq!(detect(), Language::ZhCn);
    }

    #[test]
    fn detect_no_env_returns_english() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _lcall = EnvGuard::unset("LC_ALL");
        let _lang = EnvGuard::unset("LANG");
        assert_eq!(detect(), Language::En);
    }

    #[test]
    fn detect_lang_c_returns_english() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _lcall = EnvGuard::unset("LC_ALL");
        let _lang = EnvGuard::set("LANG", "C");
        assert_eq!(detect(), Language::En);
    }

    #[test]
    fn detect_lang_posix_returns_english() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _lcall = EnvGuard::unset("LC_ALL");
        let _lang = EnvGuard::set("LANG", "POSIX");
        assert_eq!(detect(), Language::En);
    }

    #[test]
    fn detect_any_zh_prefix_is_chinese() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _lcall = EnvGuard::unset("LC_ALL");
        let _lang = EnvGuard::set("LANG", "zh_TW.Big5");
        assert_eq!(detect(), Language::ZhCn);
    }

    #[test]
    fn detect_empty_locale_falls_back_to_english() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _lcall = EnvGuard::unset("LC_ALL");
        let _lang = EnvGuard::set("LANG", "");
        assert_eq!(detect(), Language::En);
    }

    #[test]
    fn detect_empty_lc_all_falls_back_to_lang() {
        let _lock = ENV_LOCK.lock().unwrap();
        // LC_ALL 设为空字符串应视为未设置，回退查 LANG。
        let _lcall = EnvGuard::set("LC_ALL", "");
        let _lang = EnvGuard::set("LANG", "zh_CN.UTF-8");
        assert_eq!(detect(), Language::ZhCn);
    }

    #[test]
    fn detect_whitespace_only_lc_all_falls_back_to_lang() {
        let _lock = ENV_LOCK.lock().unwrap();
        // 纯空白也应视为未设置。
        let _lcall = EnvGuard::set("LC_ALL", "   ");
        let _lang = EnvGuard::set("LANG", "en_US.UTF-8");
        assert_eq!(detect(), Language::En);
    }

    // ── 默认语言 ──

    #[test]
    fn default_language_is_english() {
        assert_eq!(Language::default(), Language::En);
    }

    // ── 翻译表完整性 ──

    /// 全量 key 列表，用于 [`every_key_has_both_translations`]。
    /// 新增变体时务必同步追加，漏写会被 `translate` 的穷尽 match 拦截编译。
    fn all_keys() -> Vec<TranslationKey> {
        use TranslationKey::*;
        vec![
            // 分组标题
            GlobalGroup,
            NavigationGroup,
            WorkspacesTabsGroup,
            PanesGroup,
            CustomGroup,
            // global 分组
            PrefixMode,
            KeybindsLabel,
            Settings,
            Detach,
            ReloadConfig,
            OpenNotificationTarget,
            // navigation 分组
            Back,
            WorkspaceList,
            MoveFocus,
            CyclePane,
            OpenWorkspace,
            SwitchWorkspace,
            // workspaces / tabs 分组
            WorkspaceNavigation,
            SessionNavigator,
            NewWorkspace,
            NewWorktree,
            OpenWorktree,
            DeleteWorktreeCheckout,
            RenameWorkspace,
            CloseWorkspace,
            PreviousWorkspace,
            NextWorkspace,
            SwitchWorkspace19,
            PreviousAgent,
            NextAgent,
            FocusAgent19,
            NewTab,
            RenameTab,
            PreviousTab,
            NextTab,
            SwitchTab19,
            CloseTab,
            // panes 分组
            SplitVertical,
            SplitHorizontal,
            ClosePane,
            RenamePane,
            EditScrollback,
            CopyMode,
            ZoomPane,
            ResizeMode,
            ToggleSidebar,
            FocusPaneLeft,
            FocusPaneDown,
            FocusPaneUp,
            FocusPaneRight,
            CyclePaneNext,
            CyclePanePrevious,
            LastPane,
            // custom 分组
            CustomCommand,
            // 其余界面文案
            Unset,
            NoMatchingKeybinds,
            Close,
            PressSlashToFilter,
            FilterLabel,
            ClearLabel,
            ScrollLabel,
            SearchLabel,
            // menus.rs 模式徽章
            ModePrefix,
            ModeCopy,
            ModeNavigate,
            ModeResize,
            // PREFIX overlay
            Cancel,
            SendPrefix,
            WorkspaceNavShort,
            // COPY overlay
            EnterSearchEscCancel,
            Selecting,
            Select,
            Exit,
            ClearQExit,
            Move,
            Repeat,
            Copy,
            // NAVIGATE overlay
            WsShort,
            Pane,
            NavigatorShort,
            SplitVertSymbol,
            SplitHorizSymbol,
            Zoom,
            ResizeShort,
            UpdateReady,
            // RESIZE overlay
            Width,
            Height,
            Done,
            // global menu
            WhatsNew,
            // dialogs 通用按钮
            Save,
            Confirm,
            Open,
            CreateAndOpen,
            DeleteAnyway,
            Remove,
            // new worktree
            Branch,
            Checkout,
            Creating,
            // remove worktree
            DeleteWorktreeCheckoutQ,
            RemovesCheckoutFolder,
            BranchNotDeletedWorkspaceWillClose,
            DirtyWillBeDeleted,
            Removing,
            // open worktree
            NoMatchingWorktrees,
            FilterWorktrees,
            Checkouts,
            // confirm close
            CloseWorkspaceQ,
            CloseWorktreeGroupQ,
            PaneUnitSingular,
            PaneUnitPlural,
            WorkspaceUnitSingular,
            WorkspaceUnitPlural,
            // context menu
            CmRename,
            CmClose,
            CmCloseGroup,
            CmNewWorktree,
            CmOpenWorktree,
            CmDeleteWorktree,
            CmCollapse,
            CmExpand,
            CmNewTab,
            CmRenamePane,
            CmClearPaneName,
            CmSwapWithFocusedPane,
            CmSplitRight,
            CmSplitDown,
            CmZoom,
            CmClosePane,
            // mobile
            MobileSwitch,
            NoWorkspace,
            Filtered,
            Agents,
            NoMatchingAgents,
            Spaces,
            NewWorkspaceMobile,
            Tabs,
            NewTabMobile,
            TabPrefix,
            Menu,
            NoAgents,
            AllIdle,
            BlockedLabel,
            DoneLabel,
            WorkingLabel,
            IdleLabel,
            WaitingAgent,
            DoneAgent,
            // sidebar
            SidebarNew,
            SortGrouped,
            SortPriority,
            UnknownLabel,
            CopiedToClipboard,
            // settings
            SoundAlerts,
            SoundAlertsDesc,
            NotificationPopups,
            NotificationPopupsDesc,
            ToastInsideHerdr,
            ToastViaTerminal,
            ToastViaSystem,
            ToggleOn,
            ToggleOff,
            AgentBorderLabels,
            AgentBorderLabelsDesc,
            AgentIntegrations,
            AgentIntegrationsDesc,
            NoIntegrationTargets,
            IntegrationsHintInstall,
            IntegrationsAllInstalled,
            IntegrationsNoneFound,
            Install,
            Apply,
            SectionHint,
            // settings 分区页签
            SectionTheme,
            SectionSound,
            SectionToast,
            SectionPaneLabels,
            SectionIntegrations,
            // integration 状态
            IntegrationInstalled,
            IntegrationUpdateAvailable,
            IntegrationAvailable,
            IntegrationNotFound,
            IntegrationsAllCurrent,
        ]
    }

    /// 遍历每个变体，断言两种语言都返回非空翻译。
    #[test]
    fn every_key_has_both_translations() {
        for key in all_keys() {
            assert!(
                !translate(Language::En, key).is_empty(),
                "英文翻译缺失: {key:?}"
            );
            assert!(
                !translate(Language::ZhCn, key).is_empty(),
                "中文翻译缺失: {key:?}"
            );
        }
    }

    /// Spot-check：直接断言关键 key 的 en/zh 字面量精确值，
    /// 防止 key 错位 / value 写反 / 术语不一致。
    /// 新增 key 时在对应的 match 臂验证后可酌情补充此表。
    #[test]
    fn spot_check_translations_exact_values() {
        use TranslationKey::*;
        // (key, 英文期望, 中文期望)
        let cases: [(TranslationKey, &str, &str); 16] = [
            // 核心术语
            (Settings, "settings", "设置"),
            (Detach, "detach", "分离"),
            (Cancel, "cancel", "取消"),
            (Close, "close", "关闭"),
            // 模式徽章
            (ModePrefix, "PREFIX", "前缀"),
            (ModeCopy, "COPY", "复制"),
            // 对话框
            (CreateAndOpen, "create and open", "创建并打开"),
            (DeleteAnyway, "delete anyway", "仍然删除"),
            // context menu
            (CmRename, "Rename", "重命名"),
            (CmClosePane, "Close pane", "关闭窗格"),
            (
                CmSwapWithFocusedPane,
                "Swap with focused pane",
                "与聚焦窗格交换",
            ),
            // confirm close
            (CloseWorkspaceQ, "Close workspace?", "关闭工作区？"),
            (PaneUnitSingular, "pane", "个窗格"),
            // mobile
            (Agents, "agents", "agent"),
            (NoAgents, "no agents", "无 agent"),
            // update ready vs whats new（术语不可混淆）
            (UpdateReady, "update ready", "有更新"),
            // 末尾保留 WhatsNew 验证
        ];
        let mut checked = 0;
        for (key, en_expected, zh_expected) in cases {
            assert_eq!(
                translate(Language::En, key),
                en_expected,
                "英文 spot-check 不匹配: {key:?}"
            );
            assert_eq!(
                translate(Language::ZhCn, key),
                zh_expected,
                "中文 spot-check 不匹配: {key:?}"
            );
            checked += 1;
        }
        assert_eq!(checked, 16, "spot-check 数量不符，表可能被意外缩减");
    }
}
