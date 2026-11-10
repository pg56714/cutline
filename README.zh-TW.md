# Cutline

[English](README.md) | 繁體中文

一個小而快的截圖工具,操作流程仿照 Snipaste 的截圖模式:按下熱鍵、畫面凍結、框選區域、加上標註,然後複製、存檔或釘選。

以 Rust(egui + glow)撰寫。發佈的執行檔是單一個 exe,不需要安裝程式,也不需要額外的執行環境。

> Cutline 是獨立的專案,與 Snipaste 沒有關聯,也不是從它衍生而來;只是採用相同的大致流程。

## 功能

- **全域熱鍵**(預設 `F1`)和系統匣圖示(`Capture` / `Quit`)。
- **先凍結,再選取**:畫面在任何東西畫出來*之前*就先擷取,所以截到的就是你當下看到的樣子。
- **滑鼠移到視窗上會自動吸附**,點一下就選取該視窗,也可以拖曳選取任意區域。
- **放大鏡**:顯示游標下的像素座標與 RGB,按 `C` 可把顏色以 `#RRGGBB` 複製起來。
- **調整選區**:在選區內拖曳可移動(標註會跟著移動),拖曳 8 個控制點可縮放。
- **標註**:矩形/橢圓、箭頭/直線、畫筆、螢光筆、馬賽克、文字(支援中日韓文字)、橡皮擦。
  每種工具都有顏色與大小可選,按住 `Shift` 可限制成正方形與 45° 的線。支援復原與重做。
- **輸出**:複製到剪貼簿、存成 PNG,或把圖片**釘選**成永遠置頂的視窗
  (拖曳移動、滾輪縮放、`Ctrl+C` 複製、`Esc` / 按右鍵 / 按兩下關閉)。
- 輸出的圖片是依你看到的畫面渲染,並精準裁切到像素。

## 按鍵

| 按鍵 | 動作 |
| --- | --- |
| `F1`(可設定) | 開始截圖 |
| 點擊 / 拖曳 | 選取視窗 / 選取區域 |
| `Enter`、`Ctrl+C`、按兩下 | 複製選區並結束 |
| `Ctrl+S` | 存成 PNG |
| `F3` | 釘選選區 |
| `Ctrl+Z` / `Ctrl+Y` | 復原 / 重做 |
| 按右鍵 | 取消選區(沒有選區時則結束) |
| `Esc` | 結束 |
| `C`(選取時) | 複製游標下的顏色 |

## 設定

第一次執行時會建立 `%APPDATA%\Cutline\config.txt`:

```
hotkey = F1
```

[`global-hotkey`](https://crates.io/crates/global-hotkey) 套件看得懂的組合都可以使用,
例如 `F2` 或 `ctrl+shift+KeyA`。如果該按鍵已被其他程式佔用(例如 Snipaste),
Cutline 會提示後結束;請關閉那個程式,或改用別的按鍵。
也可以用 `CUTLINE_HOTKEY` 環境變數只在單次執行時覆寫。

## 支援的平台

| 平台 | 狀態 |
| --- | --- |
| Windows 11 | 已開發並測試,包含在乾淨的 Windows 11 虛擬機上以 100% 與 125% 顯示縮放測試(熱鍵、選取、標註、存檔、釘選、系統剪貼簿、系統匣圖示)。尚未測試 125% 以上的縮放、混合 DPI 的多螢幕,也沒有實際點過系統匣選單。 |
| Linux(X11) | 可編譯並執行(在 Xvfb 下檢查過:熱鍵、單一實例、選取、視窗吸附、標註、複製)。系統匣圖示需要桌面環境提供 status-notifier / AppIndicator。 |
| Linux(Wayland) | **目前不支援。** 畫面擷取要經過桌面的 portal,會跳出授權要求,無法照這個工具需要的方式操作;全域熱鍵與置頂的全螢幕視窗在 Wayland 上也受到限制。請使用 X11 工作階段。 |
| macOS | 通過型別檢查(`cargo check --target aarch64-apple-darwin`),但**從未實際執行過**。需要「螢幕錄製」權限。 |

## 建置

```
cargo build --release
```

**Windows**:使用預設的 `x86_64-pc-windows-msvc` 工具鏈(需要 Visual Studio Build Tools),
或使用 `x86_64-pc-windows-gnu` 目標,並讓 MinGW-w64 工具鏈在 `PATH` 上。
執行檔位於 `target/<target>/release/cutline.exe`。

**Linux**(Debian/Ubuntu 套件名稱):

```
sudo apt install build-essential pkg-config libgtk-3-dev libxdo-dev libayatana-appindicator3-dev \
  libpipewire-0.3-dev libspa-0.2-dev libclang-dev libgbm-dev libdrm-dev libxcb1-dev \
  libxcb-randr0-dev libxcb-shm0-dev libxcb-xfixes0-dev libxkbcommon-dev libdbus-1-dev \
  libegl1-mesa-dev libgl1-mesa-dev libwayland-dev
```

執行時 Linux 版需要 `libgtk-3-0`、`libayatana-appindicator3-1` 與 `libxdo3`。

`cutline --now` 會立刻截圖,不需要啟動常駐的系統匣程式。

### 測試工具

`cargo build --features devscript`(debug 版一律啟用)會編入「腳本化輸入」模式:
把 `CUTLINE_SCRIPT` 設成一串步驟,例如 `m 300 250; d; m 900 600; u; btn Shape; s shot; btn Copy`,
覆蓋層就會把它們當成合成輸入重播,並把截圖寫到 `<temp>/cutline/`。這讓整個流程不必動到真實的滑鼠就能測試。
步驟清單請見 `src/overlay/devscript.rs`。

## 運作方式

同一個執行檔,三種模式:

- **常駐**(預設):系統匣圖示與熱鍵。按下熱鍵時,擷取游標所在的螢幕,並把畫面存到暫存檔後啟動 `cutline --capture`。
- **`--capture`**:全螢幕覆蓋層。只在單次截圖期間存在。
- **`--pin`**:單一張釘選的圖片。每張釘選圖都是一個獨立的小程序。

## 已知限制

- 只會擷取游標所在的那個螢幕,不是整個虛擬桌面。
- 125% 以上的顯示縮放與混合 DPI 的多螢幕設定尚未測試。
- 沒有捲動截圖、螢幕錄影或文字辨識(OCR)。

## 參與貢獻

Commit 訊息遵循 [Conventional Commits](https://www.conventionalcommits.org/zh-hant/)
(`feat(overlay): ...`、`fix: ...`、`docs: ...`)。這不只是風格問題:發佈流程會讀取它們來決定下一個版本並產生 changelog。

## 發佈

在 Actions 頁面手動執行 **Release**。它會看上一個 tag 之後的 Conventional Commits:
`feat` 會升次版號,`fix` 與 `perf` 會升修訂號,帶 `!` 或 `BREAKING CHANGE:` 註腳的是破壞性變更
(版本低於 1.0 時升次版號)。如果沒有任何值得發佈的變更就會停止;否則它會更新 `CHANGELOG.md` 與版本號、打 tag、
建置 Windows(`.exe`)、macOS(universal `.tar.gz`)與 Linux(`.tar.gz`),等所有建置都上傳完畢後才發佈 release。
macOS 與 Linux 的版本未經簽章,測試程度也不如 Windows 版。

`bun scripts/release.ts --dry-run` 會顯示下一次發佈的內容,`bun test scripts` 會執行該腳本的測試。

## 授權

Apache License 2.0。請見 [LICENSE](LICENSE)。
