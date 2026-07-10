type Locale = 'en' | 'zh-TW';

type MessageKey =
  | 'app.title'
  | 'app.subtitle'
  | 'settings.open'
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
  | 'upload.enrichSummary'
  | 'upload.enrichTitle'
  | 'upload.enrichHint'
  | 'upload.enriching'
  | 'upload.enrichAction'
  | 'upload.continue'
  | 'upload.or'
  | 'upload.googleImport'
  | 'googleImport.title'
  | 'googleImport.description'
  | 'googleImport.cookieHint'
  | 'googleImport.cookieLabel'
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
  | 'googleImport.confirm';

const messages: Record<Locale, Record<MessageKey, string>> = {
  en: {
    'app.title': 'Mapodus',
    'app.subtitle': 'Migrate your saved lists into uMap',
    'settings.open': 'Open settings',
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
    'upload.enrichSummary': 'Enriched: {enriched}, Skipped: {skipped}',
    'upload.enrichTitle': 'Optional: Enrich with Google Maps cookies',
    'upload.enrichHint': 'If your CSV is missing coordinates or addresses, paste your Google Maps cookies to attempt automatic enrichment.',
    'upload.enriching': 'Enriching...',
    'upload.enrichAction': 'Enrich with Google Maps',
    'upload.continue': 'Continue to bookmarks',
    'upload.or': 'or',
    'upload.googleImport': 'Import directly from Google Maps',
    'googleImport.title': 'Import from Google Maps',
    'googleImport.description': 'Paste your Google cookies below. Cookies expire after a few hours, so collect fresh ones before each import.',
    'googleImport.cookieHint': 'Open DevTools (F12) -> Application -> Cookies -> https://www.google.com. Right-click any cookie -> Copy All, or copy the -b argument from a cURL command. Paste the raw cookie string here.',
    'googleImport.cookieLabel': 'Cookie string (semicolon-separated key=value pairs)',
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
  },
  'zh-TW': {
    'app.title': 'Mapodus',
    'app.subtitle': '將你儲存的清單遷移到 uMap',
    'settings.open': '開啟設定',
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
    'upload.enrichSummary': '已補足：{enriched}，略過：{skipped}',
    'upload.enrichTitle': '選用：使用 Google Maps Cookie 補足資料',
    'upload.enrichHint': '如果 CSV 缺少座標或地址，貼上 Google Maps Cookie 以嘗試自動補足。',
    'upload.enriching': '補足中...',
    'upload.enrichAction': '使用 Google Maps 補足',
    'upload.continue': '繼續選擇書籤',
    'upload.or': '或',
    'upload.googleImport': '直接從 Google Maps 匯入',
    'googleImport.title': '從 Google Maps 匯入',
    'googleImport.description': '在下方貼上 Google Cookie。Cookie 幾小時後會過期，每次匯入前請重新取得。',
    'googleImport.cookieHint': '開啟 DevTools (F12) -> Application -> Cookies -> https://www.google.com。右鍵點擊任一 Cookie -> Copy All，或從 cURL 命令複製 -b 參數，並將原始 Cookie 字串貼在這裡。',
    'googleImport.cookieLabel': 'Cookie 字串（以分號分隔的 key=value 配對）',
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
  },
};

const browserLocale = (): Locale => {
  const language = globalThis.navigator?.language.toLowerCase() ?? '';
  return language === 'zh-tw' || language.startsWith('zh-hant') ? 'zh-TW' : 'en';
};

export function t(key: MessageKey, values: Record<string, string | number> = {}): string {
  return messages[browserLocale()][key].replace(/\{(\w+)\}/g, (_, name) =>
    String(values[name] ?? `{${name}}`)
  );
}
