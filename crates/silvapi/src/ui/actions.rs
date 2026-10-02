use gpui_kit::actions;

actions!(
    silvapi,
    [
        NewCollection,
        SendRequest,
        ImportCurl,
        ImportOpenApi,
        ImportPostman,
        CancelRequest,
        ThemePicker,
        RenameSelected,
        ApiPicker,
        TabPicker,
        NextApi,
        PrevApi,
        CloseTab,
        SwitchTab1,
        SwitchTab2,
        SwitchTab3,
        SwitchTab4,
        SwitchTab5,
        SwitchTab6,
        SwitchTab7,
        SwitchTab8,
        SwitchTab9,
        SwitchLastTab,
        ToggleMaximize,
        FocusActiveRequest,
        FocusUrl,
        FocusCollectionPanel,
        FocusRequestPanel,
        FocusResponsePanel,
        OpenSettings,
        CloseSettings,
        // A no-op action used to shadow (disable) an old keybinding when a
        // shortcut is reassigned — gpui has no targeted unbind, and the
        // last-registered binding for a key wins, so binding the old key to
        // this suppresses its previous action.
        ShortcutNoOp
    ]
);
