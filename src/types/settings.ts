export interface AppSettings {
  android_sdk_path: string;
  gradle_home: string;
  java_home: string;
  node_path: string;
  cordova_path: string;
  keystore_path: string;
  keystore_alias: string;
  output_dir: string;
  build_tools_version: string;
  renpy_sdk_path: string;
}

export const DEFAULT_SETTINGS: AppSettings = {
  android_sdk_path: "",
  gradle_home: "",
  java_home: "",
  node_path: "",
  cordova_path: "",
  keystore_path: "",
  keystore_alias: "key0",
  output_dir: "",
  build_tools_version: "34.0.0",
  renpy_sdk_path: "",
};
