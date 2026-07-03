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
        OpenSettings,
        CloseSettings
    ]
);
