# Changelog

## v0.7.0 (2026-11-18)

### Features

- **macos:** explain the missing Screen Recording permission instead of freezing a black screen ([be723a3](https://github.com/pg56714/cutline/commit/be723a39477d9f3535151fa740a276ea54b98d6c))

### Fixes

- **macos:** ad-hoc sign the whole app bundle so the Screen Recording permission sticks ([5bea626](https://github.com/pg56714/cutline/commit/5bea626802e235ffdf2e52eab1399fd65b6096ea))

### Other

- update the macOS status in the READMEs now that it has been run ([2a9a631](https://github.com/pg56714/cutline/commit/2a9a631ec7f2420d2cd45911e3a03324d9913429))

## v0.6.1 (2026-11-18)

### Fixes

- **macos:** upload the screenshot texture on the first frame so Retina screens no longer crash ([9598962](https://github.com/pg56714/cutline/commit/95989625df6633e04f913f7cbfb32d1453459756))
- **overlay:** use FRAC_1_SQRT_2 instead of the 0.7071 literal that clippy denies ([cc3ed8a](https://github.com/pg56714/cutline/commit/cc3ed8a608f9cec3771e08f0a20ef762756e7eca))

### Other

- cargo fmt ([e43fb7d](https://github.com/pg56714/cutline/commit/e43fb7d65fb83c5fa6dce53d2862a1a75c8c4e6b))

## v0.6.0 (2026-11-17)

### Features

- **release:** publish SHA256SUMS and build attestations, verify downloads on update ([864cf55](https://github.com/pg56714/cutline/commit/864cf5553df87240cc8e7d87994010efbaf05189))

## v0.5.0 (2026-11-17)

### Features

- **update:** check GitHub releases and update from the tray ([b5e22b2](https://github.com/pg56714/cutline/commit/b5e22b2f5d534389308d4ec0f1ffe723623dcf9d))

## v0.4.1 (2026-11-16)

### Fixes

- **overlay:** let the IME compose in the text box ([ab5c15c](https://github.com/pg56714/cutline/commit/ab5c15c4f2dfb410bf1ad0db10df610538bed260))

## v0.4.0 (2026-11-14)

### Features

- **tray:** start at sign-in (Run key, XDG autostart, LaunchAgent) ([29beefa](https://github.com/pg56714/cutline/commit/29beefaac695b9c9aa67e4053c1fd7b6288f5d3c))

## v0.3.0 (2026-11-13)

### Features

- **overlay:** redraw the toolbar, add tooltips, polish the text box ([e104f71](https://github.com/pg56714/cutline/commit/e104f71af9151c8dd6a2ad833f09160ba8aa014d))

### Fixes

- **overlay:** avoid exclusive full screen (flash on copy / save / pin) ([537f16f](https://github.com/pg56714/cutline/commit/537f16fc561865c0dcb8e25af37bc7683038eabf))
- **pin:** outline on all four sides, square corners, exact position ([03d067a](https://github.com/pg56714/cutline/commit/03d067a0bed57958cc19bbb86f40287137dfaa3d))
- **overlay:** visible, resizable text box ([81f098b](https://github.com/pg56714/cutline/commit/81f098b5195736fe84afcb8b72595f7689e34ec5))

## v0.2.0 (2026-11-11)

### Features

- **release:** ship installers (NSIS, dmg, AppImage, deb) via cargo-packager ([0766771](https://github.com/pg56714/cutline/commit/07667717a6536a8d42492005b28f9d2286300a45))

## v0.1.2 (2026-11-11)

### Fixes

- **build:** only call embed-resource on Windows hosts ([a65eae7](https://github.com/pg56714/cutline/commit/a65eae7cfadccb49f2f4695f9510b67072f892a0))

## v0.1.1 (2026-11-11)

### Fixes

- **overlay:** remove flicker on copy/save/pin, add exe icon, Snipaste-like dimming ([73b5ae5](https://github.com/pg56714/cutline/commit/73b5ae5a1c887d2b2bb87b65dd399ca1c124002e))

### Other

- **readme:** note the Linux release is built on Ubuntu 24.04 ([275bf76](https://github.com/pg56714/cutline/commit/275bf760e765456ec2f751dfd2bb0da2f1d08917))
- **release:** build Linux on ubuntu-24.04 ([96cf4c8](https://github.com/pg56714/cutline/commit/96cf4c80bbf7e229f496d7ebe670876d0ea4c172))

## v0.1.0 (2026-11-10)

### Features

- **linux:** capture the X11 screen directly and report Wayland clearly ([3445f65](https://github.com/pg56714/cutline/commit/3445f65566f730dc91143fb69c94740a2553e5f9))
- add cutline screenshot tool ([1ebb4a6](https://github.com/pg56714/cutline/commit/1ebb4a6e06e059a82df9fe5c6fd03d0b9b93e12f))

### Other

- **readme:** update Linux support and runtime requirements ([86004cf](https://github.com/pg56714/cutline/commit/86004cf1d3daf720a4d872cdd3c1c7b3673354d5))
- **release:** build Linux on ubuntu-22.04 with the trimmed dependency list ([55648bf](https://github.com/pg56714/cutline/commit/55648bf8426ab5b5a9f026807dc58201045baeab))
- **release:** drop the test step to mirror git-tabs ([10ac6de](https://github.com/pg56714/cutline/commit/10ac6dec6ec63b57aec5033b891e591f788d6c49))
- **readme:** add a Traditional Chinese README ([91547ec](https://github.com/pg56714/cutline/commit/91547eca84706996d59c95221e0e6e1cb3bd57c5))
- drop the compile-check workflow and the test step to mirror git-tabs ([85c6dc5](https://github.com/pg56714/cutline/commit/85c6dc5b8bd6832c2d5826a81811672f3779f039))
- compile-check Windows, macOS and Linux on push and pull request ([ce28bfe](https://github.com/pg56714/cutline/commit/ce28bfe43d6fb70f3a745c45123f713ee75f1a2b))
- **readme:** document platforms, releasing and commit style ([79f8710](https://github.com/pg56714/cutline/commit/79f8710d764281494aeac3b0b147569bb03de7c4))
- **release:** manual release workflow with automatic version and changelog ([eb43cbc](https://github.com/pg56714/cutline/commit/eb43cbcc34d132b7e597d98b575c2aef51f91dc6))
- **overlay:** record real scaling in script output and allow real clipboard runs ([a9bbbdb](https://github.com/pg56714/cutline/commit/a9bbbdb0313aa1fbdfcdd1f113e03fe0f6eaba69))
- normalize line endings to LF ([5f14b0a](https://github.com/pg56714/cutline/commit/5f14b0acc37a28be8e2e2548dc0d6da9b4658ea7))

