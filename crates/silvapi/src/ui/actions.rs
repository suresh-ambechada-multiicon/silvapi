use gpui::actions;

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
        NextApi,
        PrevApi,
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
