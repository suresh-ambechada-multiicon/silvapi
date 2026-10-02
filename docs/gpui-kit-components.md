# GPUI Kit Components Catalog (v0.7.0)

This document catalogs all UI components provided by `gpui-kit` (via `gpui_kit::component`), organized by category with concise descriptions and primary usage examples.

---

## 1. Layout & Containers

| Component | Module | Description | Typical Usage |
|---|---|---|---|
| **`h_resizable` / `v_resizable`** | `resizable` | Resizable panel groups with interactive grab handles and split ratios. | Multi-pane IDE layouts, sidebar/work-area dividers. |
| **`resizable_panel`** | `resizable` | Individual panel child inside a resizable group with min/max/initial size. | Sidebar panels, bottom drawer panels. |
| **`dock`** | `dock` | Advanced IDE-style dock layout with drag-and-drop tab tearing, split panes, and auto-hiding panels. | Full workbench dock systems. |
| **`h_flex` / `v_flex`** | `gpui_kit::*` | Flex containers with gap, alignment, and distribution controls. | Horizontal toolbars, vertical stacks. |
| **`sidebar`** | `sidebar` | Collapsible navigation sidebars with headers, items, and footers. | App navigation, settings sidebars. |
| **`group_box`** | `group_box` | Bordered group container with an optional label title. | Grouping related form fields or settings. |
| **`separator`** | `separator` | Horizontal or vertical divider lines. | Menu dividers, panel separation. |
| **`WindowBorder`** | `window_border` | Native-feeling client window border and padding manager. | Frameless or custom decorated windows. |

---

## 2. Navigation & Tabs

| Component | Module | Description | Typical Usage |
|---|---|---|---|
| **`TabBar`** | `tab` | Container for multiple tabs with animated active indicators and overflow menu. | Document tabs, panel switcher. |
| **`Tab`** | `tab` | Individual tab item supporting variants (`Tab`, `Outline`, `Pill`, `Segmented`, `Underline`). | Request/Response tabs (`Params`, `Headers`, `Auth`, `Body`). |
| **`breadcrumb`** | `breadcrumb` | Breadcrumb hierarchy path with separators and clickable links. | Folder paths (`Workspace > Collection > Folder > Request`). |
| **`pagination`** | `pagination` | Page navigation controls with previous, next, and jump-to-page numbers. | Paginated API response tables. |
| **`stepper`** | `stepper` | Step-by-step progress indicator for multi-stage workflows. | Wizards, onboarding, auth setup flows. |

---

## 3. Buttons & Actions

| Component | Module | Description | Typical Usage |
|---|---|---|---|
| **`Button`** | `button` | Full-featured button supporting variants: `Primary`, `Secondary`, `Ghost`, `Danger`, `Outline`, sizes (`XSmall`, `Small`, `Medium`, `Large`), icons, and keyboard shortcuts. | Send button, New Request button, cancel action. |
| **`DropdownButton`** | `button` | Split button with a main action click and a chevron dropdown menu. | "Send" + "Send and Download". |
| **`ButtonGroup`** | `button` | Grouped set of connected buttons. | Toggle modes, segmented action triggers. |
| **`toolbar`** | `toolbar` | Specialized action bar for buttons, separators, and toggle groups. | Top bar actions, editor format bars. |

---

## 4. Text & Input Controls

| Component | Module | Description | Typical Usage |
|---|---|---|---|
| **`Input`** | `input` | Single-line text input backed by `Entity<InputState>`. Supports clear buttons, icons, password masking, placeholder, validation. | URL bar, query param keys/values, search bars. |
| **`Textarea`** | `input` | Multi-line text field backed by `Entity<TextareaState>`. Supports auto-grow, rows count, soft-wrap, and line editing. | Request body editors, raw headers editor, JSON view. |
| **`highlighter`** | `highlighter` | Syntax highlighting engine using Tree-sitter for JSON, HTTP, YAML, SQL, etc. | Highlighting JSON response bodies. |
| **`select`** | `select` | Dropdown selector backed by `Entity<SelectState<T>>` with search, selection events, and custom item rendering. | HTTP Method picker (`GET`, `POST`, etc.), Auth type selector. |
| **`combobox`** | `combobox` | Editable input coupled with a searchable dropdown list. | Auto-completing header names or environment variables. |
| **`checkbox`** | `checkbox` | Styled checkable box with indeterminate state support and labels. | Enabling/disabling query parameters and headers. |
| **`switch`** | `switch` | Animated toggle switch for boolean settings. | "Follow redirects", "SSL verification" toggles. |
| **`radio`** | `radio` | Single-selection group from multiple radio options. | Selecting body formats (`none`, `json`, `form-data`). |
| **`slider`** | `slider` | Draggable continuous or stepped range slider. | Timeout sliders, zoom levels. |
| **`color_picker`** | `color_picker` | Palette and hex color selection modal/popover. | Workspace or collection tag color customization. |

---

## 5. Data Display & Tables

| Component | Module | Description | Typical Usage |
|---|---|---|---|
| **`DataTable` / `Table`** | `table` | Virtualized grid backed by `Entity<TableState<T>>` with column resizing, sorting, selection, and cell rendering. | JSON tabular view, response history list. |
| **`tree`** | `tree` | Hierarchical collapsible tree view with indent guides and node selection. | Collections, folders, and request navigation tree. |
| **`list` / `virtual_list`** | `list` | High-performance virtualized list for thousands of items. | Request history, search results, console logs. |
| **`description_list`** | `description_list` | Key-value definition list with aligned terms and details. | Metadata inspectors, response info summaries. |
| **`badge`** | `badge` | Compact colored pill with variant colors (info, success, warning, destructive). | HTTP status codes (`200 OK`, `404`), HTTP method badges (`GET`, `POST`). |
| **`tag`** | `tag` | Removable chips with icons and labels. | Request tags, environment labels. |
| **`avatar`** | `avatar` | User profile image or fallback initials circle. | User profile, workspace avatars. |
| **`kbd`** | `kbd` | Styled keyboard shortcut badge. | Shortcut hints (`Ctrl+Enter`, `Ctrl+P`). |
| **`label`** | `label` | Typographic label with sizing, muted color variants, and weights. | Field titles, subheadings. |
| **`link`** | `link` | Underlined hyperlink element with external browser open callbacks. | Documentation links, external API references. |

---

## 6. Overlays, Dialogs & Feedback

| Component | Module | Description | Typical Usage |
|---|---|---|---|
| **`dialog`** | `dialog` | Centered modal backdrop dialog with title, body, and action buttons. | Confirm deletions, environment management modal. |
| **`sheet`** | `sheet` | Slide-in drawer from screen edge (Left, Right, Bottom, Top). | Settings drawer, history slide-over. |
| **`popover`** | `popover` | Floating contextual panel anchored to an element. | Quick variable inspector, mini settings popups. |
| **`tooltip`** | `tooltip` | Hover tooltip with delay and placement options. | Explaining button shortcuts and icons. |
| **`hover_card`** | `hover_card` | Rich hover card displaying detailed preview on element hover. | Previewing variable values on `{{variable}}` hover. |
| **`menu`** | `menu` | Context menus and dropdown menus with keyboard navigation, shortcuts, and submenus. | Right-click request actions (Duplicate, Delete, Rename). |
| **`command`** | `command` | Spotlight / Command Palette (`Ctrl+P`) with fuzzy filtering and keyboard navigation. | Fast switcher across requests, settings, and themes. |
| **`notification`** | `notification` | Toast notifications (success, error, warning, info) with auto-dismiss. | "Request copied to clipboard", "Save successful". |
| **`alert`** | `alert` | Inline callout box with icon and title. | Error banners when a request fails or times out. |
| **`empty`** | `empty` | Styled empty state with icon, title, description, and action button. | "No requests found", "No collections created yet". |
| **`spinner`** | `spinner` | Animated indeterminate loading indicator. | Request in-flight loading state. |
| **`progress`** | `progress` | Linear progress bar with percentage value. | File upload / download progress in SilvaDL or streaming. |
| **`skeleton` / `shimmer`** | `skeleton` | Animated placeholder shapes while data loads. | Content loading skeletons. |

---

## 7. Charts & Media

| Component | Module | Description | Typical Usage |
|---|---|---|---|
| **`chart` / `plot`** | `chart` | Line, bar, and scatter charts with axes, tooltips, and legends. | Response time latency waterfalls, API benchmark graphs. |
| **`carousel`** | `carousel` | Swipable slide carousel. | Feature tours, walkthroughs. |
| **`attachment`** | `attachment` | File attachment cards with upload progress and previews. | Multipart file upload attachments. |
| **`time`** | `time` | Date picker, calendar, and time field pickers. | Timestamp filters, scheduled request execution. |
