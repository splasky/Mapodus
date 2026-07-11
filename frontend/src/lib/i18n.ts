import { get, writable } from 'svelte/store';

export type Locale = 'en' | 'zh-TW';

export type MessageKey =
  | 'app.title'
  | 'app.subtitle'
  | 'common.back'
  | 'common.next'
  | 'common.previous'
  | 'about.open'
  | 'about.close'
  | 'about.title'
  | 'about.subtitle'
  | 'about.versionLabel'
  | 'about.checking'
  | 'about.updateAvailable'
  | 'about.updateUnavailable'
  | 'about.updateUnknown'
  | 'about.latestReleaseLabel'
  | 'about.releaseNotesLabel'
  | 'about.releaseNotes'
  | 'about.viewReleases'
  | 'about.creditsLabel'
  | 'about.credits'
  | 'about.legalLabel'
  | 'about.legal'
  | 'about.linksLabel'
  | 'about.website'
  | 'about.reportIssue'
  | 'settings.open'
  | 'settings.title'
  | 'settings.description'
  | 'settings.loading'
  | 'settings.savedDesktop'
  | 'settings.savedWeb'
  | 'settings.umapUrl'
  | 'settings.umapAccount'
  | 'settings.umapAccountPlaceholder'
  | 'settings.umapPassword'
  | 'settings.secretSavedPlaceholder'
  | 'settings.optionalPlaceholder'
  | 'settings.removeUmapPassword'
  | 'settings.googleMapsApiKey'
  | 'settings.removeGoogleMapsApiKey'
  | 'settings.language'
  | 'settings.devMode'
  | 'settings.desktopSecretNote'
  | 'settings.webSecretNote'
  | 'settings.saving'
  | 'settings.save'
  | 'steps.import'
  | 'steps.selectBookmarks'
  | 'steps.connectUmap'
  | 'steps.transfer'
  | 'upload.title'
  | 'upload.descriptionBeforeLink'
  | 'upload.descriptionAfterLink'
  | 'upload.takeoutLink'
  | 'upload.csvOnly'
  | 'upload.cookiesRequired'
  | 'upload.uploading'
  | 'upload.dropHint'
  | 'upload.uploaded'
  | 'upload.validationReady'
  | 'upload.validationWarning'
  | 'upload.validationMissingName'
  | 'upload.enrichSummary'
  | 'upload.enrichTitle'
  | 'upload.enrichHint'
  | 'upload.enriching'
  | 'upload.enrichAction'
  | 'upload.continue'
  | 'upload.or'
  | 'upload.googleImport'
  | 'upload.sourceHint'
  | 'googleImport.title'
  | 'googleImport.description'
  | 'googleImport.cookieHint'
  | 'googleImport.cookieLabel'
  | 'googleImport.cookieStepDevTools'
  | 'googleImport.cookieStepCopy'
  | 'googleImport.cookieStepPaste'
  | 'googleImport.cookieRequiredHint'
  | 'googleImport.fetching'
  | 'googleImport.fetch'
  | 'googleImport.selectTitle'
  | 'googleImport.selectDescription'
  | 'googleImport.selectAll'
  | 'googleImport.selectNone'
  | 'googleImport.selectedCount'
  | 'googleImport.places'
  | 'googleImport.transferMode'
  | 'googleImport.singleMap'
  | 'googleImport.perList'
  | 'googleImport.saving'
  | 'googleImport.confirm'
  | 'googleImport.selectRequiredHint'
  | 'bookmarks.title'
  | 'bookmarks.description'
  | 'bookmarks.loading'
  | 'bookmarks.selectAll'
  | 'bookmarks.selectNone'
  | 'bookmarks.selectedCount'
  | 'bookmarks.untitled'
  | 'bookmarks.hasCoordinates'
  | 'bookmarks.missingCoordinates'
  | 'bookmarks.transferring'
  | 'bookmarks.transferAction'
  | 'connect.title'
  | 'connect.description'
  | 'connect.missingRequired'
  | 'connect.missingWithoutSavedPassword'
  | 'connect.umapUrl'
  | 'connect.username'
  | 'connect.usernamePlaceholder'
  | 'connect.password'
  | 'connect.savedPasswordPlaceholder'
  | 'connect.passwordPlaceholder'
  | 'connect.connecting'
  | 'connect.connect'
  | 'transfer.title'
  | 'transfer.progress'
  | 'transfer.createdMaps'
  | 'transfer.openInUmap'
  | 'transfer.success'
  | 'transfer.mapId'
  | 'transfer.starting'
  | 'transfer.uploadAnother';

const messages: Record<Locale, Record<MessageKey, string>> = {
  en: {
    'app.title': 'Mapodus',
    'app.subtitle': 'Migrate your saved lists into uMap',
    'common.back': 'Back',
    'common.next': 'Next',
    'common.previous': 'Previous',
    'about.open': 'About',
    'about.close': 'Close about',
    'about.title': 'About Mapodus',
    'about.subtitle': 'Google Maps Saved Lists to uMap',
    'about.versionLabel': 'Version',
    'about.checking': 'Checking...',
    'about.updateAvailable': 'Update available',
    'about.updateUnavailable': 'Could not check releases',
    'about.updateUnknown': 'Not checked yet',
    'about.latestReleaseLabel': 'Latest release',
    'about.releaseNotesLabel': 'Release notes',
    'about.releaseNotes': 'Feature descriptions are listed on the GitHub releases page.',
    'about.viewReleases': 'View releases',
    'about.creditsLabel': 'Credits',
    'about.credits': 'HY Chang (splasky)',
    'about.legalLabel': 'License',
    'about.legal': 'This software is free software. Feel free to use, modify, and distribute.',
    'about.linksLabel': 'Links',
    'about.website': 'Repository',
    'about.reportIssue': 'Report an Issue',
    'settings.open': 'Open settings',
    'settings.title': 'Settings',
    'settings.description': 'Configure defaults used during migration. Passwords and API keys are never shown after saving.',
    'settings.loading': 'Loading settings...',
    'settings.savedDesktop': 'Settings saved. Secrets are stored in the OS credential vault.',
    'settings.savedWeb': 'Settings saved. Secrets are session-only in web/server mode.',
    'settings.umapUrl': 'uMap URL',
    'settings.umapAccount': 'uMap account',
    'settings.umapAccountPlaceholder': 'optional uMap username',
    'settings.umapPassword': 'uMap password',
    'settings.secretSavedPlaceholder': 'Saved. Enter a new value to replace it.',
    'settings.optionalPlaceholder': 'Optional',
    'settings.removeUmapPassword': 'Remove saved uMap password',
    'settings.googleMapsApiKey': 'Google Maps API key',
    'settings.removeGoogleMapsApiKey': 'Remove saved Google Maps API key',
    'settings.language': 'Language',
    'settings.devMode': 'Enable developer mode',
    'settings.desktopSecretNote': 'Sensitive values are stored with your OS credential vault/keychain.',
    'settings.webSecretNote': 'Web/server mode keeps sensitive values in this browser session only.',
    'settings.saving': 'Saving...',
    'settings.save': 'Save settings',
    'steps.import': 'Import',
    'steps.selectBookmarks': 'Select bookmarks',
    'steps.connectUmap': 'Connect uMap',
    'steps.transfer': 'Transfer',
    'upload.title': 'Upload Google Takeout CSV',
    'upload.descriptionBeforeLink': 'Download your saved places from ',
    'upload.descriptionAfterLink': ' and upload the CSV file here.',
    'upload.takeoutLink': 'Google Takeout',
    'upload.csvOnly': 'Please upload a CSV file (Google Takeout)',
    'upload.cookiesRequired': 'Paste your cookies first',
    'upload.uploading': 'Uploading...',
    'upload.dropHint': 'Drag & drop your CSV file here, or click to browse',
    'upload.uploaded': 'Uploaded {count} places',
    'upload.validationReady': 'All {total} places have coordinates and are ready for uMap',
    'upload.validationWarning': '{ready} of {total} places ready - {missingCoords} missing coordinates',
    'upload.validationMissingName': ', {count} missing title',
    'upload.enrichSummary': 'Enriched: {enriched}, Skipped: {skipped}',
    'upload.enrichTitle': 'Optional: Enrich with Google Maps cookies',
    'upload.enrichHint': 'If your CSV is missing coordinates or addresses, paste your Google Maps cookies to attempt automatic enrichment.',
    'upload.enriching': 'Enriching...',
    'upload.enrichAction': 'Enrich with Google Maps',
    'upload.continue': 'Continue to bookmarks',
    'upload.or': 'or',
    'upload.googleImport': 'Import directly from Google Maps',
    'upload.sourceHint': 'Continue becomes available after one import source has produced bookmarks.',
    'googleImport.title': 'Import from Google Maps',
    'googleImport.description': 'Paste your Google cookies below. Cookies expire after a few hours, so collect fresh ones before each import.',
    'googleImport.cookieHint': 'Open DevTools (F12) -> Application -> Cookies -> https://www.google.com. Right-click any cookie -> Copy All, or copy the -b argument from a cURL command. Paste the raw cookie string here.',
    'googleImport.cookieLabel': 'Cookie string (semicolon-separated key=value pairs)',
    'googleImport.cookieStepDevTools': 'Open Google Maps in your browser, then open DevTools -> Application -> Cookies -> https://www.google.com.',
    'googleImport.cookieStepCopy': 'Copy the Google cookie string. You can use Copy All from the cookie table or copy the -b value from a cURL request.',
    'googleImport.cookieStepPaste': 'Paste the raw semicolon-separated key=value string below. Do not edit or remove cookie names.',
    'googleImport.cookieRequiredHint': 'Paste cookies to enable saved-list import.',
    'googleImport.fetching': 'Fetching lists...',
    'googleImport.fetch': 'Fetch my saved lists',
    'googleImport.selectTitle': 'Select lists to import',
    'googleImport.selectDescription': 'Choose which saved lists to import from Google Maps.',
    'googleImport.selectAll': 'Select all',
    'googleImport.selectNone': 'Select none',
    'googleImport.selectedCount': '{selected} / {total} lists selected',
    'googleImport.places': '{count} places',
    'googleImport.transferMode': 'Transfer mode:',
    'googleImport.singleMap': 'All in one map',
    'googleImport.perList': 'One map per list',
    'googleImport.saving': 'Saving...',
    'googleImport.confirm': 'Import selected to uMap',
    'googleImport.selectRequiredHint': 'Select at least one list to continue.',
    'bookmarks.title': 'Select Bookmarks',
    'bookmarks.description': 'Choose which places to transfer to uMap.',
    'bookmarks.loading': 'Loading bookmarks...',
    'bookmarks.selectAll': 'Select All',
    'bookmarks.selectNone': 'Select None',
    'bookmarks.selectedCount': '{selected} / {total} selected',
    'bookmarks.untitled': 'Untitled',
    'bookmarks.hasCoordinates': 'Has coordinates',
    'bookmarks.missingCoordinates': 'Missing coordinates',
    'bookmarks.transferring': 'Enriching & transferring...',
    'bookmarks.transferAction': 'Transfer {count} bookmarks to uMap',
    'connect.title': 'Connect to uMap',
    'connect.description': 'Enter your uMap instance URL and login credentials.',
    'connect.missingRequired': 'Please fill in uMap URL and username',
    'connect.missingWithoutSavedPassword': 'Please fill in all fields',
    'connect.umapUrl': 'uMap URL',
    'connect.username': 'Username',
    'connect.usernamePlaceholder': 'your uMap username',
    'connect.password': 'Password',
    'connect.savedPasswordPlaceholder': 'Saved password will be used if left blank',
    'connect.passwordPlaceholder': 'your uMap password',
    'connect.connecting': 'Connecting...',
    'connect.connect': 'Connect',
    'transfer.title': 'Transfer to uMap',
    'transfer.progress': 'Creating map and uploading bookmarks...',
    'transfer.createdMaps': 'Created {count} maps:',
    'transfer.openInUmap': 'Open in uMap',
    'transfer.success': 'Map created successfully!',
    'transfer.mapId': 'Map ID: {id}',
    'transfer.starting': 'Starting transfer...',
    'transfer.uploadAnother': 'Upload another map 🗺️!',
  },
  'zh-TW': {
    'app.title': 'Mapodus',
    'app.subtitle': '將你儲存的清單遷移到 uMap',
    'common.back': '返回',
    'common.next': '下一步',
    'common.previous': '上一步',
    'about.open': '關於',
    'about.close': '關閉關於視窗',
    'about.title': '關於 Mapodus',
    'about.subtitle': 'Google Maps 儲存清單到 uMap',
    'about.versionLabel': '版本',
    'about.checking': '檢查中...',
    'about.updateAvailable': '有更新版本',
    'about.updateUnavailable': '無法檢查發佈版本',
    'about.updateUnknown': '尚未檢查',
    'about.latestReleaseLabel': '最新版本',
    'about.releaseNotesLabel': '發佈說明',
    'about.releaseNotes': '功能描述列在 GitHub releases 頁面。',
    'about.viewReleases': '查看發佈版本',
    'about.creditsLabel': '製作者',
    'about.credits': 'HY Chang (splasky)',
    'about.legalLabel': '授權',
    'about.legal': '本軟體為自由軟體。你可以自由使用、修改和發佈。',
    'about.linksLabel': '連結',
    'about.website': '專案儲存庫',
    'about.reportIssue': '回報問題',
    'settings.open': '開啟設定',
    'settings.title': '設定',
    'settings.description': '設定遷移時使用的預設值。密碼與 API key 儲存後不會再次顯示。',
    'settings.loading': '正在載入設定...',
    'settings.savedDesktop': '設定已儲存。機密資料已存入作業系統憑證庫。',
    'settings.savedWeb': '設定已儲存。Web/server 模式只會在此瀏覽器工作階段保留機密資料。',
    'settings.umapUrl': 'uMap URL',
    'settings.umapAccount': 'uMap 帳號',
    'settings.umapAccountPlaceholder': '選填的 uMap 使用者名稱',
    'settings.umapPassword': 'uMap 密碼',
    'settings.secretSavedPlaceholder': '已儲存。輸入新值即可取代。',
    'settings.optionalPlaceholder': '選填',
    'settings.removeUmapPassword': '移除已儲存的 uMap 密碼',
    'settings.googleMapsApiKey': 'Google Maps API key',
    'settings.removeGoogleMapsApiKey': '移除已儲存的 Google Maps API key',
    'settings.language': '語言',
    'settings.devMode': '啟用開發者模式',
    'settings.desktopSecretNote': '機密資料會儲存在作業系統憑證庫/鑰匙圈。',
    'settings.webSecretNote': 'Web/server 模式只會在此瀏覽器工作階段保留機密資料。',
    'settings.saving': '儲存中...',
    'settings.save': '儲存設定',
    'steps.import': '匯入',
    'steps.selectBookmarks': '選擇書籤',
    'steps.connectUmap': '連線 uMap',
    'steps.transfer': '轉移',
    'upload.title': '上傳 Google Takeout CSV',
    'upload.descriptionBeforeLink': '從 ',
    'upload.descriptionAfterLink': ' 下載已儲存的地點，然後在這裡上傳 CSV 檔。',
    'upload.takeoutLink': 'Google Takeout',
    'upload.csvOnly': '請上傳 Google Takeout CSV 檔',
    'upload.cookiesRequired': '請先貼上 Cookie',
    'upload.uploading': '上傳中...',
    'upload.dropHint': '將 CSV 檔拖曳到這裡，或點擊瀏覽檔案',
    'upload.uploaded': '已上傳 {count} 個地點',
    'upload.validationReady': '全部 {total} 個地點都有座標，可以匯入 uMap',
    'upload.validationWarning': '{total} 個地點中有 {ready} 個可用 - {missingCoords} 個缺少座標',
    'upload.validationMissingName': '，{count} 個缺少標題',
    'upload.enrichSummary': '已補足：{enriched}，略過：{skipped}',
    'upload.enrichTitle': '選用：使用 Google Maps Cookie 補足資料',
    'upload.enrichHint': '如果 CSV 缺少座標或地址，貼上 Google Maps Cookie 以嘗試自動補足。',
    'upload.enriching': '補足中...',
    'upload.enrichAction': '使用 Google Maps 補足',
    'upload.continue': '繼續選擇書籤',
    'upload.or': '或',
    'upload.googleImport': '直接從 Google Maps 匯入',
    'upload.sourceHint': '當任一匯入來源產生書籤後，才能繼續下一步。',
    'googleImport.title': '從 Google Maps 匯入',
    'googleImport.description': '在下方貼上 Google Cookie。Cookie 幾小時後會過期，每次匯入前請重新取得。',
    'googleImport.cookieHint': '開啟 DevTools (F12) -> Application -> Cookies -> https://www.google.com。右鍵點擊任一 Cookie -> Copy All，或從 cURL 命令複製 -b 參數，並將原始 Cookie 字串貼在這裡。',
    'googleImport.cookieLabel': 'Cookie 字串（以分號分隔的 key=value 配對）',
    'googleImport.cookieStepDevTools': '在瀏覽器開啟 Google Maps，然後開啟 DevTools -> Application -> Cookies -> https://www.google.com。',
    'googleImport.cookieStepCopy': '複製 Google Cookie 字串。可以從 Cookie 表格使用 Copy All，或從 cURL request 複製 -b 的值。',
    'googleImport.cookieStepPaste': '將原始的分號分隔 key=value 字串貼到下方。不要編輯或移除 Cookie 名稱。',
    'googleImport.cookieRequiredHint': '貼上 Cookie 後才能匯入儲存清單。',
    'googleImport.fetching': '正在取得清單...',
    'googleImport.fetch': '取得我的儲存清單',
    'googleImport.selectTitle': '選擇要匯入的清單',
    'googleImport.selectDescription': '選擇要從 Google Maps 匯入的儲存清單。',
    'googleImport.selectAll': '全選',
    'googleImport.selectNone': '全不選',
    'googleImport.selectedCount': '已選擇 {selected} / {total} 個清單',
    'googleImport.places': '{count} 個地點',
    'googleImport.transferMode': '轉移模式：',
    'googleImport.singleMap': '全部放在同一張地圖',
    'googleImport.perList': '每個清單一張地圖',
    'googleImport.saving': '儲存中...',
    'googleImport.confirm': '匯入選取清單到 uMap',
    'googleImport.selectRequiredHint': '至少選擇一個清單才能繼續。',
    'bookmarks.title': '選擇書籤',
    'bookmarks.description': '選擇要轉移到 uMap 的地點。',
    'bookmarks.loading': '正在載入書籤...',
    'bookmarks.selectAll': '全選',
    'bookmarks.selectNone': '全不選',
    'bookmarks.selectedCount': '已選擇 {selected} / {total} 個',
    'bookmarks.untitled': '未命名',
    'bookmarks.hasCoordinates': '有座標',
    'bookmarks.missingCoordinates': '缺少座標',
    'bookmarks.transferring': '正在補足並轉移...',
    'bookmarks.transferAction': '轉移 {count} 個書籤到 uMap',
    'connect.title': '連線到 uMap',
    'connect.description': '輸入你的 uMap 網站 URL 與登入資訊。',
    'connect.missingRequired': '請填寫 uMap URL 與使用者名稱',
    'connect.missingWithoutSavedPassword': '請填寫所有欄位',
    'connect.umapUrl': 'uMap URL',
    'connect.username': '使用者名稱',
    'connect.usernamePlaceholder': '你的 uMap 使用者名稱',
    'connect.password': '密碼',
    'connect.savedPasswordPlaceholder': '留空時會使用已儲存的密碼',
    'connect.passwordPlaceholder': '你的 uMap 密碼',
    'connect.connecting': '連線中...',
    'connect.connect': '連線',
    'transfer.title': '轉移到 uMap',
    'transfer.progress': '正在建立地圖並上傳書籤...',
    'transfer.createdMaps': '已建立 {count} 張地圖：',
    'transfer.openInUmap': '在 uMap 開啟',
    'transfer.success': '地圖建立成功！',
    'transfer.mapId': '地圖 ID：{id}',
    'transfer.starting': '正在開始轉移...',
    'transfer.uploadAnother': '上傳另一張地圖 🗺️!',
  },
};

export const locale = writable<Locale>(browserLocale());

function browserLocale(): Locale {
  return normalizeLocale(globalThis.navigator?.language) ?? 'en';
}

export function normalizeLocale(value: string | null | undefined): Locale | null {
  const language = value?.trim().toLowerCase().replace('_', '-');
  if (!language) return null;
  if (language === 'zh-tw' || language.startsWith('zh-hant')) return 'zh-TW';
  if (language === 'en' || language.startsWith('en-')) return 'en';
  return null;
}

export function setLocale(value: string | null | undefined): void {
  locale.set(normalizeLocale(value) ?? browserLocale());
}

export function t(key: MessageKey, values: Record<string, string | number> = {}): string {
  const activeLocale = get(locale);
  const template = messages[activeLocale]?.[key] ?? messages.en[key];
  return template.replace(/\{(\w+)\}/g, (_, name) => String(values[name] ?? `{${name}}`));
}
