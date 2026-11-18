# Changelog

## v0.7.0 (2026-11-18)

### Features

- **macos:** explain the missing Screen Recording permission instead of freezing a black screen ([2cf7115](https://github.com/pg56714/cutline/commit/2cf71159e25d6bf8e20e51af8bb8c0edb76d11ba))

### Fixes

- **macos:** ad-hoc sign the whole app bundle so the Screen Recording permission sticks ([698a7f3](https://github.com/pg56714/cutline/commit/698a7f36011edff1116684b3a0dc58628cd51cc8))

### Other

- update the macOS status in the READMEs now that it has been run ([df4c6d4](https://github.com/pg56714/cutline/commit/df4c6d4df0e3c2c792894f9a8a76b9a9af1e5a5b))

## v0.6.1 (2026-11-18)

### Fixes

- **macos:** upload the screenshot texture on the first frame so Retina screens no longer crash ([7f30842](https://github.com/pg56714/cutline/commit/7f30842bb4d529ac59019597223b85b20cebb814))
- **overlay:** use FRAC_1_SQRT_2 instead of the 0.7071 literal that clippy denies ([d045b89](https://github.com/pg56714/cutline/commit/d045b89af5ad0c8d2ee6c5e3b6e72dc5af3bd39c))

### Other

- cargo fmt ([cfad438](https://github.com/pg56714/cutline/commit/cfad43806e0e14c58087546133e6d5bce9d06c78))

## v0.6.0 (2026-11-17)

### Features

- **release:** publish SHA256SUMS and build attestations, verify downloads on update ([c9866e2](https://github.com/pg56714/cutline/commit/c9866e27ceb40a74a0f3d70c8edd2552374d742d))

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

- **build:** only call embed-resource on Windows hosts ([95d77ee](https://github.com/pg56714/cutline/commit/95d77ee0753829fb54f28ae9a905cf02122a52e5))

## v0.1.1 (2026-11-11)

### Fixes

- **overlay:** remove flicker on copy/save/pin, add exe icon, Snipaste-like dimming ([a361fa2](https://github.com/pg56714/cutline/commit/a361fa2ce01d3dee09acd50243cf824321a4c4aa))

### Other

- **readme:** note the Linux release is built on Ubuntu 24.04 ([6a5a33c](https://github.com/pg56714/cutline/commit/6a5a33cad69c109ca09a7e1e1663eabd89eef639))
- **release:** build Linux on ubuntu-24.04 ([6546923](https://github.com/pg56714/cutline/commit/6546923fca4a37a26315f6e5104490b6838b3305))

## v0.1.0 (2026-11-10)

### Features

- **linux:** capture the X11 screen directly and report Wayland clearly ([c8eadcb](https://github.com/pg56714/cutline/commit/c8eadcb3bf11d0b438489746cf2e22f7e1512e74))
- add cutline screenshot tool ([0f1e1bb](https://github.com/pg56714/cutline/commit/0f1e1bbd17290ab0b466338fc4a6254c32c19def))

### Other

- **readme:** update Linux support and runtime requirements ([8fa8447](https://github.com/pg56714/cutline/commit/8fa844795227a1bb727e70bb60723bcd2cee3dec))
- **release:** build Linux on ubuntu-22.04 with the trimmed dependency list ([fcbc8c9](https://github.com/pg56714/cutline/commit/fcbc8c90e1f7f6588921c154051822547ea7c2d1))
- **release:** drop the test step to mirror git-tabs ([6f119c0](https://github.com/pg56714/cutline/commit/6f119c00f73f760a1d8a2d6942de5527791e737e))
- **readme:** add a Traditional Chinese README ([9cfc698](https://github.com/pg56714/cutline/commit/9cfc698935bb3779a262bad4ff4cdd67374bb2e3))
- drop the compile-check workflow and the test step to mirror git-tabs ([23c3ab4](https://github.com/pg56714/cutline/commit/23c3ab482cab4a289f2acd652e9455ea226f2dbe))
- compile-check Windows, macOS and Linux on push and pull request ([69c0ad6](https://github.com/pg56714/cutline/commit/69c0ad683c8a34afa7bac909d02fd5629703db3a))
- **readme:** document platforms, releasing and commit style ([53ff14a](https://github.com/pg56714/cutline/commit/53ff14a2f4208162212be0a1b209b78374e7df11))
- **release:** manual release workflow with automatic version and changelog ([c41c9cf](https://github.com/pg56714/cutline/commit/c41c9cf30b13923fef629758779371fbc82729b5))
- **overlay:** record real scaling in script output and allow real clipboard runs ([f259a24](https://github.com/pg56714/cutline/commit/f259a240616dfb8270739014d7c8c8cc28d0bbb6))
- normalize line endings to LF ([f71409a](https://github.com/pg56714/cutline/commit/f71409a134893788b248a4d16beb0281e439c746))

