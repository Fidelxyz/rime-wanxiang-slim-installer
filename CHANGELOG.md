# Changelog

## [1.4.1](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/compare/v1.4.0...v1.4.1) (2026-11-18)


### Bug Fixes

* show actual SHA-256 values on digest mismatch ([bb8dd66](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/commit/bb8dd6661ca4703d52a59235b2be6c469e4ce452))


### Performance Improvements

* check grammar update against asset timestamp without a request ([7425be6](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/commit/7425be6bc7e94aee0b6762759fb4bf324fe1c7f1))
* compute SHA-256 while downloading ([74767b5](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/commit/74767b5c7f37318a69b2fe3bc195a717f3c44e9f))
* move blocking file I/O off the async download task ([9bc6bcd](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/commit/9bc6bcd94198d8300a52ebadf81517d5ca34b002))

## [1.4.0](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/compare/v1.3.3...v1.4.0) (2026-11-12)


### Features

* download all files before confirming installation ([d4304b7](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/commit/d4304b7e9282d2bb762ba95b86240c973969e7ac))
* remove stale YAML files when installing schema ([078c9b3](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/commit/078c9b3edd8218fc29993b007449e9cda833ebf7))


### Performance Improvements

* download schema and grammar in parallel ([974f13d](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/commit/974f13d57474962304f2834fa4a893114c8e3acc))

## [1.3.3](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/compare/v1.3.2...v1.3.3) (2026-10-30)


### Performance Improvements

* reduce binary size by trimming dependency features ([1c6063e](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/commit/1c6063e81fecacabe5364d634959a6a9fcdc20c8))

## [1.3.2](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/compare/v1.3.1...v1.3.2) (2026-10-18)


### Bug Fixes

* restore HTTPS support for downloads ([c650022](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/commit/c650022c54e7bce656ed015c9f188641cb9b3c4e))

## [1.3.1](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/compare/v1.3.0...v1.3.1) (2026-10-15)


### Build System

* **deps:** update dependencies ([9df5e2d](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/commit/9df5e2df1d7fc3183950a86053f63c2abc7e4e6e))

## [1.3.0](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/compare/v1.2.0...v1.3.0) (2026-09-27)


### Features

* release file locks before installing grammar model on Windows ([c9c1b40](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/commit/c9c1b4040b6c36af8712e78013ed16f7ebfb93fc))


### Bug Fixes

* exit silently when installation is cancelled ([56ccef3](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/commit/56ccef3425f01685300f2a4aad727ef71523b43b))
* print warning to stderr ([9df2da1](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/commit/9df2da1b849dfc1010f41cf548a1d564756ccad5))

## [1.2.0](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/compare/v1.1.0...v1.2.0) (2026-09-20)


### Features

* show breaking changes when checking updates ([c425a60](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/commit/c425a60174af871385fd5d7064a53f626da113ab))

## [1.1.0](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/compare/v1.0.1...v1.1.0) (2026-09-20)


### Features

* add prerelease update toggle ([35b448d](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/commit/35b448d0104e8f72ca3e2d2fc845f2ca2b78449a))


### Bug Fixes

* clean up temporary files on SIGINT ([d5ef6b1](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/commit/d5ef6b12b2ff5323ca13c8d12ba2a2fb331ac233))

## [1.0.1](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/compare/v1.0.0...v1.0.1) (2026-09-18)


### Bug Fixes

* fix inconsistent schema name ([74b6697](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/commit/74b6697ec4223dd32b68df40634ab30fb07272a0))

## 1.0.0 (2026-09-18)


### Features

* basic installer ([3af7069](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/commit/3af7069ebacd98d4d5157d4a96d8454077dd8a77))


### Continuous Integration

* fix release workflow ([9a484cc](https://github.com/Fidelxyz/rime-wanxiang-slim-installer/commit/9a484cc1c7a19a88d2e251b493b88a00fa6a9f21))
