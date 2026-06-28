# Google Cloud Project 設定指南

本指南說明如何建立 Google Cloud 專案、設定 OAuth 2.0 同意畫面，並取得用戶端 ID 與密鑰，以供本機開發環境使用。

---

## 1. 建立 GCP 專案

1. 開啟 [Google Cloud Console](https://console.cloud.google.com/)
2. 點擊頂端專案選擇器 → **New Project**
3. 填寫專案名稱（例如 `google-maps-to-umap`）
4. 無須連結帳單帳戶
5. 建立後，**切換到新專案**（從頂端選擇器）

---

## 2. 開啟 Google Auth Platform

1. 從左側漢堡選單 → **APIs & Services → OAuth consent screen**
2. 如果是全新專案，會顯示「Google Auth Platform not configured」，點擊 **Get started** 進入精靈

---

## 3. 設定 OAuth 同意畫面

精靈共有 4 個步驟：

### 3.1 App Information
- **App name**: `google-maps-to-umap`（顯示給使用者的名稱）
- **User support email**: 選取你的 Google 帳號
- **Logo**: 可略過

### 3.2 Audience
- 選取 **External**（任何 Google 帳號均可登入）
  - Internal 僅限 Google Workspace 組織內部使用
  - 選錯無法事後修改，需開新專案
- 點擊 **Next**

### 3.3 Contact Information
- 填入開發者聯絡 email

### 3.4 確認並建立

完成後會進入同意畫面總覽頁面。

---

## 4. 建立 OAuth 用戶端 ID

1. 在左側導覽列 → **APIs & Services → Credentials**
2. 點擊 **Create Credentials → OAuth client ID**
3. **Application type**: 選取 **Web application**
4. **Name**: 輸入名稱（例如 `google-maps-to-umap-dev`）
5. **Authorized JavaScript origins**:
   - `http://localhost:8900`
6. **Authorized redirect URIs**:
   - `http://localhost:8900/api/auth/google/callback`
7. 點擊 **Create**

完成後會彈出對話框顯示 **Client ID** 與 **Client Secret**，請複製並妥善保管。

---

## 5. 設定環境變數

將取得的憑證寫入專案根目錄的 `.env` 檔案：

```env
GOOGLE_CLIENT_ID=your-client-id.apps.googleusercontent.com
GOOGLE_CLIENT_SECRET=your-client-secret
```

> **安全注意**：`.env` 已加入 `.gitignore`，請勿將憑證提交到版本控制。

---

## 6. 啟動驗證

```bash
cargo run -p web
# → 開啟 http://localhost:3000
# → 點擊「Sign in with Google」
# → 應跳轉至 Google 登入頁面
```

---

## 7. 生產環境部署

部署至正式網域時，需在 Credentials 頁面更新：

- **Authorized JavaScript origins**: 加入正式網域（如 `https://yourdomain.com`）
- **Authorized redirect URIs**: 加入 `https://yourdomain.com/api/auth/google/callback`
- 若需要使用超過 100 個測試使用者，需將同意畫面從 **Testing** 發布為 **In production**

---

## 注意事項

| 項目 | 說明 |
|------|------|
| 不需啟用任何 Google API | 僅使用 OAuth 2.0 登入，無需啟用 Gmail、Drive 等 API |
| 不需帳單帳戶 | OAuth 同意畫面與用戶端 ID 完全免費 |
| 測試模式限制 | 最多 100 個測試使用者，正式發布後取消此限制 |
| 敏感範圍 | 若後續使用 Data Portability API（`dataportability.maps.starred_places`），需額外申請 Google 審查 |
