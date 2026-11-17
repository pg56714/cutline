# Cutline

[English](README.md) | 繁體中文

一個小而快的截圖工具,操作流程仿照 Snipaste 的截圖模式:按下熱鍵、畫面凍結、框選區域、加上標註,然後複製、存檔或釘選。

以 Rust(egui + glow)撰寫。發佈的是單一個 exe(也有安裝程式可選),不需要額外的執行環境。

> Cutline 是獨立的專案,與 Snipaste 沒有關聯,也不是從它衍生而來;只是採用相同的大致流程。

> [!WARNING]
> **應用程式沒有程式碼簽章**,第一次執行會被系統擋下,只需處理一次:
> - **Windows**:「Windows 已保護您的電腦」→ **其他資訊** → **仍要執行**
> - **macOS**:系統設定 → 隱私權與安全性 → 找到 Cutline,按 **仍要開啟**;或在終端機執行 `xattr -cr /Applications/Cutline.app`
> - **Linux**:AppImage 先執行 `chmod +x`;.deb 照一般方式安裝
>
> 因為沒有簽章,請自行驗證下載的檔案:見[驗證下載的檔案](#驗證下載的檔案)。

## 下載

從[最新版本](https://github.com/pg56714/cutline/releases/latest)下載對應系統的檔案。

| 系統 | 檔案 | 說明 |
| --- | --- | --- |
| Windows | `cutline_*_x64-setup.exe` | 安裝程式 |
| Windows | `cutline_windows_x64.exe` | 免安裝,直接執行 |
| macOS | `Cutline_*.dmg` | Apple Silicon |
| Linux | `cutline_*_x86_64.AppImage` | 先執行 `chmod +x` |
| Linux | `cutline_*_amd64.deb` | Debian / Ubuntu:`sudo apt install ./cutline_*.deb` |

## 功能

- **全域熱鍵**(預設 `F1`)和系統匣圖示(`Capture` / `Start with Windows` / `Quit`)。
- **先凍結,再選取**:畫面在任何東西畫出來*之前*就先擷取,所以截到的就是你當下看到的樣子。
- **滑鼠移到視窗上會自動吸附**,點一下就選取該視窗,也可以拖曳選取任意區域。
- **放大鏡**:顯示游標下的像素座標與 RGB,按 `C` 可把顏色以 `#RRGGBB` 複製起來。
- **調整選區**:在選區內拖曳可移動(標註會跟著移動),拖曳 8 個控制點可縮放。
- **標註**:矩形/橢圓、箭頭/直線、畫筆、螢光筆、馬賽克、文字(支援中日韓文字)、橡皮擦。
  每種工具都有顏色與大小可選,按住 `Shift` 可限制成正方形與 45° 的線。支援復原與重做。
  滑鼠停在工具列按鈕上會顯示它的功能與快捷鍵。
- **文字**:輸入框會跟著文字變寬,到選區邊緣自動換行;拖曳右邊緣可設定寬度,拖曳外框可移動位置,
  輸入途中選大小或顏色會直接套用到正在輸入的文字。
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

## 登入時自動啟動

在系統匣選單勾選 **Start with Windows**(Linux 與 macOS 上是 **Start at login**),登入時就會自動啟動 Cutline;
取消勾選就不再自動啟動。預設為關閉。這個設定存放在系統本身保存它的地方,而不是 `config.txt`:

| 平台 | 位置 |
| --- | --- |
| Windows | `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run` 底下的 `Cutline` 值 |
| Linux | `~/.config/autostart/cutline.desktop`(有設定 `$XDG_CONFIG_HOME` 時是 `$XDG_CONFIG_HOME/autostart/cutline.desktop`)。AppImage 會透過 `.AppImage` 檔本身啟動。 |
| macOS | `~/Library/LaunchAgents/app.cutline.desktop.plist` |

- 啟動的是勾選它的那一份 Cutline。如果搬動了免安裝版的 exe,或執行的是另一份 Cutline,
  那裡的選項會顯示未勾選;勾選後就改成啟動那一份。
- Cutline 啟動時才讀取這個設定,所以在系統自己的設定裡關閉後,下次啟動時選項就會顯示未勾選:
  Windows 是「工作管理員 → 啟動應用程式」或「設定 → 應用程式 → 啟動」,Linux 是桌面環境的自動啟動設定。
  macOS 例外:在「系統設定 → 一般 → 登入項目」裡關閉後,選項仍會顯示勾選。
- 解除安裝 Cutline 不會移除這個設定,請先取消勾選。殘留的設定指向已不存在的檔案,不會啟動任何東西。
- Windows 上的登錄檔處理有單元測試涵蓋,但還沒實際測試勾選後重新登入;Linux 與 macOS 的部分都還沒測試。

## 更新

Cutline 啟動時會安靜地檢查一次有沒有新的[版本](https://github.com/pg56714/cutline/releases),之後只有按 **Check for updates** 才會再檢查(沒有定時檢查)。
有新版時,托盤選單的 **Check for updates** 會變成 **Update to vX.Y.Z**,點它就會更新。
沒有新版時點它,會立刻檢查並告訴你結果。

| 你怎麼執行它 | 更新會做什麼 |
| --- | --- |
| Windows 安裝版 | 下載新的安裝程式並執行 |
| Windows 免安裝 `.exe`、Linux AppImage | 下載新檔案、原地換掉,然後重新啟動 Cutline |
| `.deb`、`.dmg`、自己編譯的 | 開啟版本頁面,請自行安裝 |

- 下載透過 `curl`,Windows 10 以上、macOS 和桌面版 Linux 都已內建。下載到的若不是程式(中斷或錯誤頁面),不會取代任何東西。版本若附有 `SHA256SUMS`,下載的檔案還必須與它相符;更早、沒有此檔的版本照舊接受。
- 設定 `CUTLINE_NO_UPDATE_CHECK=1` 可關閉檢查(托盤項目也會停用)。
- 換檔需要對檔案所在資料夾有寫入權限。
- 已測試:版本比較、挑對檔案、下載與換檔(Windows 與 Ubuntu 的單元測試),以及 Linux 上有新版時常駐程式能正常啟動。托盤項目的點擊尚未實測。

## 驗證下載的檔案

每個版本都附有 `SHA256SUMS` 檔和建置證明(GitHub 簽發的紀錄,證明檔案是由本專案的發佈流程建置)。兩者任一都能確認檔案就是流程產出的那一個。

```powershell
# Windows:和 SHA256SUMS 裡你那個檔案的那一行比對
Get-FileHash .\cutline_windows_x64.exe -Algorithm SHA256
```

```bash
# macOS / Linux,SHA256SUMS 放在同一個資料夾
sha256sum -c --ignore-missing SHA256SUMS      # macOS:shasum -a 256 -c --ignore-missing SHA256SUMS

# 更可靠:需要 GitHub CLI
gh attestation verify <檔案> --repo pg56714/cutline
```

`SHA256SUMS` 和檔案放在同一個版本頁面,所以能發現檔案損毀或被掉包,但擋不住 GitHub 帳號被入侵;建置證明才擋得住。兩者都不能取代程式碼簽章,所以上面的系統警告依然會出現。

## 支援的平台

| 平台 | 狀態 |
| --- | --- |
| Windows 11 | 已開發並測試,包含在乾淨的 Windows 11 虛擬機上以 100% 與 125% 顯示縮放測試(熱鍵、選取、標註、存檔、釘選、系統剪貼簿、系統匣圖示)。尚未測試 125% 以上的縮放、混合 DPI 的多螢幕,也沒有實際點過系統匣選單。 |
| Linux(X11) | 已在 Ubuntu 22.04 與 24.04 上編譯並執行(在虛擬 X 伺服器下檢查過:熱鍵、單一實例、選取、視窗吸附、標註、複製)。預先建置的發佈版是在 Ubuntu 24.04(glibc 2.39)上建置的,所以需要同樣新的系統;較舊的系統(例如 Ubuntu 22.04)請自行從原始碼建置(已確認可以)。系統匣圖示需要桌面環境提供 status-notifier / AppIndicator。尚未在真實的桌面工作階段中測試。 |
| Linux(Wayland) | **目前不支援。** Cutline 是透過 X 伺服器擷取畫面,在 Wayland 上只會得到全黑或不完整的圖片,所以改為顯示提示訊息。Wayland 上的畫面擷取必須經過桌面的 portal,會跳出授權要求;全域熱鍵與置頂的全螢幕視窗也受到限制。請使用 X11 工作階段。設定 `CUTLINE_FORCE_X11=1` 可略過檢查,但只能透過 XWayland 擷取 X11 的視窗。 |
| macOS | 通過型別檢查(`cargo check --target aarch64-apple-darwin`),但**從未實際執行過**。需要「螢幕錄製」權限。 |

## 建置

Rust 版本固定在 `rust-toolchain.toml`(發佈流程用同一個版本),新的 stable 出來也不會自己改變建置結果。要升級時,
同時改那裡的 `channel` 和 `.github/workflows/release.yml` 裡 `dtolnay/rust-toolchain@` 的標籤。

```
cargo build --release
```

**Windows**:使用預設的 `x86_64-pc-windows-msvc` 工具鏈(需要 Visual Studio Build Tools),
或使用 `x86_64-pc-windows-gnu` 目標,並讓 MinGW-w64 工具鏈在 `PATH` 上。
執行檔位於 `target/<target>/release/cutline.exe`。

**Linux**(Debian/Ubuntu 套件名稱):

```
sudo apt install build-essential pkg-config libgtk-3-dev libxdo-dev libayatana-appindicator3-dev
```

Linux 上是直接和 X 伺服器溝通來擷取畫面(`src/capture_x11.rs`),所以不需要任何 pipewire / Wayland 的開發套件。
這也是能在 Ubuntu 22.04 上建置的原因:xcap 在 Linux 上會拉進 pipewire 的綁定,而它需要比 22.04 內建更新的 libpipewire。

執行時 Linux 版需要 `libgtk-3-0`、`libayatana-appindicator3-1`、`libxdo3` 與 `libxkbcommon-x11-0`
(桌面版 Ubuntu 都有,最小安裝可能沒有),以及一般的 OpenGL(Mesa)函式庫。

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
建置 Windows(安裝程式與免安裝 `.exe`)、macOS(universal `.dmg`)與 Linux(`.AppImage` 與 `.deb`),等所有建置都上傳完畢後才發佈 release。
macOS 與 Linux 的版本未經簽章,測試程度也不如 Windows 版。

`bun scripts/release.ts --dry-run` 會顯示下一次發佈的內容,`bun test scripts` 會執行該腳本的測試。

## 授權

Apache License 2.0。請見 [LICENSE](LICENSE)。
