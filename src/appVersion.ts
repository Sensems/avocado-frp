import packageJson from '../package.json'

/** Product version from package.json (kept aligned with tauri.conf via check:versions). */
export const APP_VERSION: string = packageJson.version
