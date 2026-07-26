# POS UI bundle

`pos.pos-ui` owns all POS UI code and produces the deployable static artifact:

```sh
cd ../pos.pos-ui
yarn bundle
```

This writes `artifacts/pos-ui-bundle.zip`. The Tauri host consumes it during a package build:

```sh
cd ../pos.pos-ui-app
yarn build
```

For CI or a release artifact hosted elsewhere, set `POS_UI_BUNDLE` to the absolute path of the downloaded zip before running `yarn build`. The bundle is unpacked only into the ignored `bundle/` directory and is then embedded by Tauri.
