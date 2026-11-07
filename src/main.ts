import './style.css';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

type Provider = 'mymemory' | 'siliconflow' | 'aliyun' | 'deepseek' | 'openrouter' | 'openai' | 'zhipu' | 'moonshot' | 'gemini' | 'anthropic' | 'volcengine';
type Settings = { enabled: boolean; excluded_apps: string[]; has_key: boolean; provider: Provider; model: string; source_language: string; target_language: string; capture_error?: string | null; credential_error?: string | null };
type FreeModel = { id: string; name: string };
type SelectionEvent = { text?: string };
type Page = 'translate' | 'discover' | 'settings';

const app = document.querySelector<HTMLDivElement>('#app')!;
const windowKind = new URLSearchParams(location.search).get('window') || 'main';
const isTauri = '__TAURI_INTERNALS__' in window;
const icons = {
  spark: '<svg viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="m12 2 1.8 5.7L19.5 9.5l-5.7 1.8L12 17l-1.8-5.7L4.5 9.5l5.7-1.8L12 2ZM19 15l.9 2.6 2.6.9-2.6.9L19 22l-.9-2.6-2.6-.9 2.6-.9L19 15Z" fill="currentColor"/></svg>',
  translate: '<svg viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M3 6h11M8.5 3v3m3 0c-.5 4-3.2 7-7.5 8.5m1.5-5c1.4 2 3.4 3.7 6 5M14 20l3.5-9 3.5 9m-6-2.5h5" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/></svg>',
  compass: '<svg viewBox="0 0 24 24" fill="none" aria-hidden="true"><circle cx="12" cy="12" r="8.5" stroke="currentColor" stroke-width="1.7"/><path d="m15.7 8.3-2.2 5.2-5.2 2.2 2.2-5.2 5.2-2.2Z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"/></svg>',
  settings: '<svg viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M12 3v2m0 14v2M3 12h2m14 0h2M5.6 5.6 7 7m10 10 1.4 1.4m0-12.8L17 7M7 17l-1.4 1.4" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/><circle cx="12" cy="12" r="4.5" stroke="currentColor" stroke-width="1.7"/></svg>',
  arrow: '<svg viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M5 12h14m-5-5 5 5-5 5" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/></svg>',
  external: '<svg viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="M13 5h6v6M19 5l-8 8m5 1v4H6V8h4" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/></svg>',
  copy: '<svg viewBox="0 0 24 24" fill="none" aria-hidden="true"><rect x="8" y="8" width="11" height="11" rx="2" stroke="currentColor" stroke-width="1.6"/><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2" stroke="currentColor" stroke-width="1.6"/></svg>',
  close: '<svg viewBox="0 0 24 24" fill="none" aria-hidden="true"><path d="m6 6 12 12M18 6 6 18" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/></svg>',
};
const urls = {
  siliconSignup: 'https://cloud.siliconflow.cn/', siliconPrice: 'https://siliconflow.cn/pricing',
  aliPrice: 'https://help.aliyun.com/zh/machine-translation/product-overview/billing-overview',
  openrouter: 'https://openrouter.ai/collections/free-models', aliSignup: 'https://mt.console.aliyun.com/', routerSignup: 'https://openrouter.ai/signup',
  mymemoryLimits: 'https://mymemory.translated.net/doc/usagelimits.php', mymemoryPrivacy: 'https://mymemory.translated.net/terms-and-conditions',
  bailianConsole: 'https://bailian.console.aliyun.com/',
  bailianPrice: 'https://help.aliyun.com/zh/model-studio/model-pricing',
  bailianFree: 'https://help.aliyun.com/zh/model-studio/new-free-quota',
  deepseek: 'https://platform.deepseek.com/',
  openai: 'https://platform.openai.com/',
  openrouterKeys: 'https://openrouter.ai/settings/keys',
  zhipu: 'https://open.bigmodel.cn/',
  moonshot: 'https://platform.kimi.com/',
  gemini: 'https://aistudio.google.com/',
  anthropic: 'https://platform.claude.com/',
  volcengine: 'https://console.volcengine.com/ark',
};
type CloudProvider = Exclude<Provider, 'mymemory'>;
type ProviderInfo = { name: string; mark: string; signup: string; description: string };
const cloudProviders: Record<CloudProvider, ProviderInfo> = {
  aliyun: { name: '阿里云百炼', mark: '阿', signup: urls.bailianConsole, description: '北京地域。符合条件的账户可享试用额度，到期或用完可能扣费；请在控制台核对额度并开启「免费额度用完即停」。模型列表需工作空间 ID，默认模型可直接使用，也可手动填写。' },
  siliconflow: { name: '硅基流动', mark: 'S', signup: urls.siliconSignup, description: '默认翻译模型当前标价免费；其他模型的价格和调用权限请在硅基流动核对。' },
  deepseek: { name: 'DeepSeek', mark: 'D', signup: urls.deepseek, description: '使用 DeepSeek 开放平台的 API Key；模型价格、可用性和账户余额以平台为准。' },
  openrouter: { name: 'OpenRouter', mark: 'O', signup: urls.openrouterKeys, description: '可选择免费或收费模型；账号权限、网络及模型实际价格以 OpenRouter 为准。' },
  zhipu: { name: '智谱 AI', mark: '智', signup: urls.zhipu, description: '使用智谱开放平台 API Key；请手动填写模型 ID。模型价格和额度以平台为准。' },
  moonshot: { name: '月之暗面', mark: '月', signup: urls.moonshot, description: '使用 Kimi 中国站 API Key；国际站 Key 不通用。模型价格和额度以平台为准。' },
  volcengine: { name: '火山方舟', mark: '火', signup: urls.volcengine, description: '使用火山方舟北京地域 API Key；模型可能需要在控制台单独开通，费用以平台为准。' },
  openai: { name: 'OpenAI', mark: 'O', signup: urls.openai, description: '使用 OpenAI API Key；模型访问权限、地区和价格以 OpenAI 为准。' },
  gemini: { name: 'Google Gemini', mark: 'G', signup: urls.gemini, description: '使用 Gemini API Key；可用模型、地区和免费额度以 Google 为准。' },
  anthropic: { name: 'Anthropic Claude', mark: 'A', signup: urls.anthropic, description: '使用 Anthropic API Key；模型权限、网络及价格以 Anthropic 为准。' },
};
let settings: Settings = { enabled: true, excluded_apps: [], has_key: false, provider: 'siliconflow', model: 'tencent/Hunyuan-MT-7B', source_language: 'en', target_language: 'zh' };
let page: Page = 'translate';
let freeModels: FreeModel[] = [];
let modelsLoaded = false;
let noticeTimer: ReturnType<typeof setTimeout> | undefined;
let selectionGeneration = 0;
let lastResult = '';
let lastSource = '';
let loadError = '';
const availableModels: Partial<Record<Provider, FreeModel[]>> = {};
let modelStatus = '';
let modelStatusError = false;
let modelLoading = false;
let workspaceId = '';
const escapeHtml = (value: unknown) => String(value).replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#039;' }[c]!));
const errorMessage = (error: unknown) => error instanceof Error ? error.message : typeof error === 'string' ? error : '操作失败，请稍后重试';

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri) return invoke<T>(command, args);
  // The browser preview is deliberately read-only: it never pretends to call a translation service.
  if (command === 'get_settings') return { enabled: true, excluded_apps: [], has_key: false, provider: 'siliconflow', model: 'tencent/Hunyuan-MT-7B', source_language: 'en', target_language: 'zh' } as T;
  if (command === 'open_url') { window.open(String(args?.url), '_blank', 'noopener,noreferrer'); return undefined as T; }
  throw new Error('浏览器预览模式：请在桌面应用中使用此功能');
}
function notify(message: string, kind: 'success' | 'error' = 'success') {
  const toast = document.querySelector<HTMLElement>('#toast');
  if (!toast) return;
  toast.textContent = message;
  toast.dataset.kind = kind;
  toast.classList.add('show');
  clearTimeout(noticeTimer);
  noticeTimer = setTimeout(() => toast.classList.remove('show'), 4000);
}
function brand() { return `<span class="brand-mark">${icons.spark}</span><span class="brand-name">一划<span class="brand-dot">.</span></span>`; }
function navigation() {
  const item = (id: Page, text: string, icon: string) => `<button class="nav-item ${page === id ? 'active' : ''}" data-page="${id}" ${page === id ? 'aria-current="page"' : ''}>${icon}<span>${text}</span>${page === id ? '<span class="nav-active-dot"></span>' : ''}</button>`;
  return `<aside class="sidebar"><div class="brand">${brand()}</div><div class="side-caption">工作空间</div><nav class="nav">${item('translate', '开始翻译', icons.translate)}${item('discover', '免费接口发现', icons.compass)}${item('settings', '偏好设置', icons.settings)}</nav><div class="side-bottom"><div class="side-tip"><span class="side-tip-icon">✳</span><div><strong>轻一点，也懂得多一点。</strong><p>选中内容，点击图标即可翻译。</p></div></div><span class="version">为阅读而生 <span>·</span> Windows</span></div></aside>`;
}
function statusMarkup() { const problem = loadError || settings.capture_error; return `<span class="status-dot ${problem ? 'failed' : !isTauri || !settings.enabled ? 'off' : ''}"></span>${problem ? '划词状态异常' : !isTauri ? '浏览器预览' : settings.enabled ? '划词已开启' : '划词已暂停'}`; }
function captureWarning() { const problem = loadError || settings.capture_error; return problem ? `<div class="capture-warning" role="alert"><strong>划词状态异常</strong><span>${escapeHtml(problem)}</span></div>` : ''; }
function providerPicker() {
  const options = Object.entries(cloudProviders).map(([id, info]) => `<option value="${id}" ${settings.provider === id ? 'selected' : ''}>${escapeHtml(info.name)}</option>`).join('');
  return `<div class="provider-selector"><label class="input-label" for="provider-select">翻译服务</label><select id="provider-select">${options}<option value="mymemory" ${settings.provider === 'mymemory' ? 'selected' : ''}>MyMemory · 免密钥</option></select></div>`;
}
function modelChoices(provider: Provider) {
  return (availableModels[provider] || []).map(model => `<option value="${escapeHtml(model.id)}" label="${escapeHtml(model.name || model.id)}"></option>`).join('');
}
function modelControls() {
  const isAli = settings.provider === 'aliyun';
  const isZhipu = settings.provider === 'zhipu';
  return `<div class="model-config"><div class="model-heading"><div><label class="input-label" for="model-id">使用模型</label><p>当前模型：<strong id="active-model">${escapeHtml(settings.model || '未设置')}</strong></p></div></div><div class="model-input-row"><input id="model-id" list="provider-model-options" value="${escapeHtml(settings.model || '')}" autocomplete="off" spellcheck="false" placeholder="输入或从列表中选择模型 ID"><datalist id="provider-model-options">${modelChoices(settings.provider)}</datalist><button class="button secondary" data-action="save-model">使用模型</button></div>${isAli ? `<div class="workspace-input"><label class="input-label" for="workspace-id">工作空间 ID <span>· 仅拉取百炼模型列表时需要</span></label><input id="workspace-id" value="${escapeHtml(workspaceId)}" autocomplete="off" spellcheck="false" placeholder="llm-…"></div>` : ''}<div class="model-actions">${isZhipu ? '<span>智谱暂不支持获取模型列表，请在上方输入模型 ID。</span>' : `<button class="button secondary" data-action="fetch-provider-models" ${modelLoading ? 'disabled' : ''}>${modelLoading ? '正在获取…' : '获取模型列表'}</button><span>输入模型名可搜索；部分列表含非文本模型。</span>`}</div><p id="provider-model-status" class="model-status ${modelStatusError ? 'error' : ''}" role="status">${escapeHtml(modelStatus)}</p><p class="model-footnote">实际调用权限与费用以服务商为准。获取列表不会发起翻译。</p></div>`;
}
function providerDetails() {
  if (settings.provider === 'mymemory') {
    return `<div class="provider-head"><div class="provider-icon">M</div><div class="provider-title"><strong>MyMemory</strong><p>英文、中文、日文短文本翻译</p></div><span class="connection connected"><i></i>已选择</span></div><p class="provider-description">无需注册；匿名额度为每日 5,000 字符，单次最多 500 UTF-8 字节、一个段落。额度与网络出口相关，可能已用尽；请在偏好设置中选择原文语言。</p><div class="provider-privacy"><strong>请勿翻译敏感内容。</strong>点击翻译后原文会发送至 MyMemory；服务商称提交的文本片段可能长期保存，并可能与其合作方共享。<button class="text-link" data-url="${urls.mymemoryPrivacy}">隐私条款 ${icons.external}</button><button class="text-link" data-url="${urls.mymemoryLimits}">额度说明 ${icons.external}</button></div>`;
  }
  const provider = settings.provider as CloudProvider;
  const info = cloudProviders[provider];
  if (!info) return `<p class="error">当前服务无法识别，请重新选择。</p>`;
  const links = provider === 'aliyun' ? `<span class="provider-links"><button class="text-link" data-url="${urls.bailianFree}">额度与用完即停 ${icons.external}</button><button class="text-link" data-url="${urls.bailianPrice}">模型价格 ${icons.external}</button></span>` : '';
  return `<div class="provider-head"><div class="provider-icon ${provider === 'aliyun' ? 'ali' : ''}">${escapeHtml(info.mark)}</div><div class="provider-title"><strong>${escapeHtml(info.name)}</strong><p>点击翻译后原文发往 ${escapeHtml(info.name)}</p></div><span class="connection ${settings.has_key ? 'connected' : ''}"><i></i>${settings.has_key ? '已保存密钥' : '尚未配置'}</span></div><div class="provider-description">${escapeHtml(info.description)}${links}</div>${settings.credential_error ? `<p class="error small" role="alert">${escapeHtml(settings.credential_error)}</p>` : ''}<label class="input-label" for="api-key">${escapeHtml(info.name)} API Key</label><div class="key-row"><div class="key-input-wrap"><input id="api-key" type="password" autocomplete="off" spellcheck="false" placeholder="${settings.has_key ? '已保存，输入新密钥可替换' : `粘贴你的${escapeHtml(info.name)} API Key`}"/><button class="key-visibility" data-action="toggle-key" aria-label="显示密钥" title="显示密钥">显示</button></div><button class="button primary" data-action="save-key">保存密钥</button>${settings.has_key ? '<button class="button secondary" data-action="clear-key">清除</button>' : ''}</div><div class="key-footer"><span>密钥分别保存在 Windows 凭据管理器中</span><button class="text-link" data-url="${info.signup}">申请 API Key ${icons.external}</button></div>${modelControls()}`;
}
function mainPage() {
  return `<div class="page-head"><div><div class="eyebrow">YOUR READING COMPANION</div><h1>读下去，<span>不必停。</span></h1><p class="lead">选中不懂的句子，点一下图标，译文就在眼前。</p></div><span class="top-status">${statusMarkup()}</span></div>
  ${captureWarning()}<section class="hero" aria-label="划词翻译使用方法"><div class="hero-copy"><div class="hero-kicker"><span class="mini-spark">✳</span> 简单三步</div><h2>让语言的距离，<br>只有<span>一划。</span></h2><p>在浏览器或文档中选中文字，点击出现的图标，即可查看翻译。</p><button class="text-link light-link" data-action="to-settings">管理划词设置 ${icons.arrow}</button></div><div class="hero-art" aria-hidden="true"><div class="paper-line short"></div><div class="paper-line"></div><div class="sample-selection">A little curiosity goes a long way.<span class="sample-bubble">${icons.translate}</span></div><div class="paper-line medium"></div><div class="sample-card"><span>中</span><div><small>译文</small><strong>一点好奇心，可以走很远。</strong></div></div><div class="hero-star one">✳</div><div class="hero-star two">✳</div></div></section>
  <div class="section-top"><div><span class="section-kicker">READY WHEN YOU ARE</span><h2>配置翻译服务</h2></div><button class="text-link" data-action="to-discover">寻找免费接口 ${icons.arrow}</button></div>
  <section class="provider-card">${providerPicker()}${providerDetails()}</section>
  <section class="manual"><div><span class="section-kicker">PASTE & TRANSLATE</span><h2>手动翻译</h2><p>遇到无法划词的应用，可以在这里粘贴文字。</p></div><label class="sr-only" for="manual-text">输入需要翻译的文字</label><textarea id="manual-text" rows="3" placeholder="在这里粘贴或输入文字…"></textarea><div class="manual-actions"><span>内容仅在点击翻译后发送</span><button class="button primary" data-action="manual-translate">翻译文字 ${icons.arrow}</button></div><div id="manual-result" class="manual-result" hidden></div></section>`;
}
function discoverPage() {
  return `<div class="page-head"><div><div class="eyebrow">FIND YOUR API</div><h1>好用的接口，<span>慢慢找。</span></h1><p class="lead">从官方入口开始，找适合自己的免费翻译服务。</p></div><span class="top-status">资料核对于 2026.11.07</span></div><div class="info-band"><span>✳</span><p><strong>免费政策会变。</strong>使用前请核对服务商的最新价格和账户额度；应用不会自动切换到收费模型。</p></div><div class="catalog-head"><div><span class="section-kicker">THE SHORTLIST</span><h2>精选入口</h2></div><span class="muted small">官方资料 · 非实时价格</span></div>
  <section class="catalog"><article class="catalog-card"><div class="catalog-top"><div class="provider-icon">M</div><div><h3>MyMemory <span class="pill">无需密钥</span></h3><p>短文本翻译 · 匿名使用</p></div><span class="free-label">每日免费额度</span></div><p class="catalog-desc">每天 5,000 字符，每次最多 500 UTF-8 字节、一个段落。文本会发送至服务商；服务商称提交文本可能长期保存并与合作方共享，请勿发送敏感内容。</p><div class="catalog-actions"><button class="button dark" data-action="use-mymemory">选择 MyMemory ${icons.arrow}</button><button class="text-link" data-url="${urls.mymemoryLimits}">额度说明 ${icons.external}</button><button class="text-link" data-url="${urls.mymemoryPrivacy}">隐私条款 ${icons.external}</button></div></article><article class="catalog-card"><div class="catalog-top"><div class="provider-icon">S</div><div><h3>硅基流动</h3><p>Hunyuan-MT-7B · 翻译模型</p></div><span class="free-label">当前标价免费</span></div><p class="catalog-desc">模型输入、输出在官方价格页标为免费。需注册、实名认证，免费模型有速率限制。</p><div class="catalog-actions"><button class="button dark" data-action="use-siliconflow">配置使用 ${icons.arrow}</button><button class="text-link" data-url="${urls.siliconPrice}">官方价格 ${icons.external}</button></div></article>
  <article class="catalog-card"><div class="catalog-top"><div class="provider-icon ali">阿</div><div><h3>阿里云百炼 · DeepSeek</h3><p>DeepSeek-V4.1-Flash · 北京地域</p></div><span class="free-label warm">限时试用额度</span></div><p class="catalog-desc">符合条件的账户可享 90 天试用额度；超出后可能扣费，建议开启「免费额度用完即停」。</p><div class="catalog-actions"><button class="button dark" data-action="use-aliyun">配置使用 ${icons.arrow}</button><button class="text-link" data-url="${urls.bailianFree}">额度说明 ${icons.external}</button><button class="text-link" data-url="${urls.bailianPrice}">模型价格 ${icons.external}</button></div></article>
  <article class="catalog-card"><div class="catalog-top"><div class="provider-icon ali">阿</div><div><h3>阿里云机器翻译</h3><p>通用版 · 按字符计费</p></div><span class="free-label warm">每月免费额度</span></div><p class="catalog-desc">官方列有每月 100 万字符免费额度，超额可能自动进入后付费。此处提供发现入口。</p><div class="catalog-actions"><button class="text-link" data-url="${urls.aliPrice}">查看官方价格 ${icons.external}</button><button class="text-link" data-url="${urls.aliSignup}">前往申请 ${icons.external}</button></div></article>
  <article class="catalog-card"><div class="catalog-top"><div class="provider-icon router">O</div><div><h3>OpenRouter</h3><p>免费大模型 · 适合探索</p></div><span class="free-label">免费模型</span></div><p class="catalog-desc">免费模型有调用次数限制；实际可用性及数据政策以官方页面和账户为准。</p><div class="catalog-actions"><button class="button dark" data-action="use-openrouter">配置使用 ${icons.arrow}</button><button class="button secondary" data-action="refresh-models">刷新免费模型</button><button class="text-link" data-url="${urls.openrouter}">官方说明 ${icons.external}</button><button class="text-link" data-url="${urls.routerSignup}">前往注册 ${icons.external}</button></div><div id="model-list" class="model-list">${modelList()}</div></article></section>`;
}
function modelList() {
  if (!modelsLoaded) return '';
  if (!freeModels.length) return '<p class="muted small">暂未发现符合条件的免费文本模型。</p>';
  return `<div class="model-list-title">本次发现 <strong>${freeModels.length}</strong> 个候选 <span>· 价格与可用性请再次核对官方页面</span></div><div class="model-tags">${freeModels.map(m => `<span title="${escapeHtml(m.id)}">${escapeHtml(m.name || m.id)}</span>`).join('')}</div>`;
}
function settingsPage() {
  const options = (selected: string) => `<option value="en" ${selected === 'en' ? 'selected' : ''}>英语</option><option value="zh" ${selected === 'zh' ? 'selected' : ''}>中文</option><option value="ja" ${selected === 'ja' ? 'selected' : ''}>日语</option>`;
  return `<div class="page-head"><div><div class="eyebrow">MAKE IT YOURS</div><h1>安静地，<span>刚刚好。</span></h1><p class="lead">把划词体验调整成适合你的样子。</p></div><span class="top-status">${statusMarkup()}</span></div>${captureWarning()}<section class="settings-card"><div class="setting-row"><div><h3>自动划词</h3><p>选中文字后显示翻译图标。暂停后仍可手动翻译。</p></div><label class="switch"><input type="checkbox" id="enabled" ${settings.enabled ? 'checked' : ''}/><span></span><span class="sr-only">开启自动划词</span></label></div><div class="setting-row column"><div><h3>排除应用</h3><p>在不希望显示图标的程序中暂停划词，每行一个程序名，例如 <code>Code.exe</code>。</p></div><label class="sr-only" for="excluded-apps">排除应用，每行一个</label><textarea id="excluded-apps" rows="4" placeholder="例如：&#10;WindowsTerminal.exe&#10;KeePass.exe">${escapeHtml(settings.excluded_apps.join('\n'))}</textarea></div><div class="setting-row column"><div><h3>原文语言</h3><p>MyMemory 使用所选原文语言；大模型会根据文本识别语言。</p></div><label class="sr-only" for="source-language">原文语言</label><select id="source-language">${options(settings.source_language)}</select></div><div class="setting-row column"><div><h3>目标语言</h3><p>选择翻译结果使用的语言；使用 MyMemory 时请与原文语言选不同的语言。</p></div><label class="sr-only" for="target-language">目标语言</label><select id="target-language">${options(settings.target_language)}</select></div><div class="settings-actions"><button class="button primary" data-action="save-settings">保存设置</button></div></section><div class="privacy-note"><span>✳</span><div><strong>你的选择，你来决定。</strong><p>应用会在本机读取选中文本以显示图标；点击翻译时，原文才会发送到已配置的翻译服务。</p></div></div>`;
}
function renderMain() {
  app.innerHTML = `<div class="app-shell">${navigation()}<main class="main-content"><div class="main-inner">${page === 'translate' ? mainPage() : page === 'discover' ? discoverPage() : settingsPage()}</div><footer class="footer"><span>读得专注，想得更远。</span><span>YIHUA / 一划</span></footer></main></div><div id="toast" class="toast" role="status" aria-live="polite"></div>${!isTauri ? '<div class="preview-badge">浏览器预览</div>' : ''}`;
}
function go(next: Page) { page = next; renderMain(); document.querySelector('.main-content')?.scrollTo(0, 0); }
function requireElement<T extends HTMLElement>(selector: string): T { const el = document.querySelector<T>(selector); if (!el) throw new Error('页面已切换，请重试'); return el; }
async function saveKey() {
  const input = requireElement<HTMLInputElement>('#api-key'); const apiKey = input.value.trim();
  if (!apiKey) { input.focus(); notify('请先输入 API Key', 'error'); return; }
  const provider = settings.provider;
  await call('set_api_key', { apiKey, provider }); delete availableModels[provider];
  if (settings.provider === provider) { settings.has_key = true; settings.credential_error = null; modelStatus = ''; renderAtCurrentScroll(); notify('密钥已保存'); }
}
function renderAtCurrentScroll() {
  const top = document.querySelector<HTMLElement>('.main-content')?.scrollTop || 0;
  renderMain(); document.querySelector<HTMLElement>('.main-content')?.scrollTo(0, top);
}
async function selectProvider(provider: Provider) {
  if (provider !== settings.provider) {
    const { enabled, excluded_apps, source_language, target_language } = settings;
    await call('save_settings', { settings: { enabled, excluded_apps, source_language, target_language, provider } });
    try { settings = await call<Settings>('get_settings'); }
    catch (error) { settings = { ...settings, provider, has_key: false, credential_error: `无法读取密钥状态：${errorMessage(error)}` }; }
  }
  modelStatus = ''; modelStatusError = false; modelLoading = false;
  if (page === 'translate') renderAtCurrentScroll(); else go('translate');
}
async function saveProviderModel(button: HTMLButtonElement) {
  const input = requireElement<HTMLInputElement>('#model-id'); const model = input.value.trim();
  if (!model) { input.focus(); notify('请输入模型 ID', 'error'); return; }
  const provider = settings.provider; button.disabled = true;
  try {
    await call('save_model', { provider, model });
    if (settings.provider === provider && page === 'translate') {
      settings.model = model; requireElement<HTMLElement>('#active-model').textContent = model; notify('模型已保存');
    }
  } finally { button.disabled = false; }
}
async function fetchProviderModels(button: HTMLButtonElement) {
  const provider = settings.provider;
  const status = requireElement<HTMLElement>('#provider-model-status');
  if (!settings.has_key) { status.textContent = '请先保存该服务的 API Key。'; status.classList.add('error'); return; }
  workspaceId = document.querySelector<HTMLInputElement>('#workspace-id')?.value.trim() || '';
  modelLoading = true; modelStatus = '正在获取模型列表…'; modelStatusError = false;
  status.textContent = modelStatus; status.classList.remove('error'); button.disabled = true; button.textContent = '正在获取…';
  try {
    const models = await call<FreeModel[]>('fetch_models', { provider, ...(provider === 'aliyun' ? { workspaceId: workspaceId || null } : {}) });
    availableModels[provider] = models;
    if (settings.provider === provider && page === 'translate') {
      modelStatus = models.length ? `已获取 ${models.length} 个模型。输入筛选并选择后，点击「使用模型」。` : '该账户未返回模型；可手动输入模型 ID。';
      requireElement<HTMLDataListElement>('#provider-model-options').innerHTML = modelChoices(provider);
      const currentStatus = requireElement<HTMLElement>('#provider-model-status'); currentStatus.textContent = modelStatus; currentStatus.classList.remove('error');
    }
  } catch (error) {
    if (settings.provider === provider && page === 'translate') {
      modelStatus = `获取失败：${errorMessage(error)}；仍可手动输入模型 ID。`; modelStatusError = true;
      const currentStatus = requireElement<HTMLElement>('#provider-model-status'); currentStatus.textContent = modelStatus; currentStatus.classList.add('error');
    }
  } finally {
    if (settings.provider === provider) { modelLoading = false; const currentButton = document.querySelector<HTMLButtonElement>('[data-action=fetch-provider-models]'); if (currentButton) { currentButton.disabled = false; currentButton.textContent = '获取模型列表'; } }
  }
}
async function saveSettings() {
  const enabled = requireElement<HTMLInputElement>('#enabled').checked;
  const excluded_apps = requireElement<HTMLTextAreaElement>('#excluded-apps').value.split(/[\n,，]/).map(s => s.trim()).filter(Boolean);
  const source_language = requireElement<HTMLSelectElement>('#source-language').value;
  const target_language = requireElement<HTMLSelectElement>('#target-language').value;
  await call('save_settings', { settings: { enabled, excluded_apps, source_language, target_language, provider: settings.provider } });
  try { settings = await call<Settings>('get_settings'); loadError = ''; } catch (error) { settings = { ...settings, enabled, excluded_apps, source_language, target_language }; loadError = `设置已保存，但无法读取应用状态：${errorMessage(error)}`; } renderMain(); notify(loadError || '设置已保存', loadError ? 'error' : 'success');
}
async function manualTranslate(button: HTMLButtonElement) {
  const text = requireElement<HTMLTextAreaElement>('#manual-text').value.trim();
  if (!text) { notify('请输入要翻译的文字', 'error'); return; }
  const result = requireElement<HTMLDivElement>('#manual-result'); result.hidden = false; result.classList.remove('error'); result.textContent = '正在翻译…';
  button.disabled = true;
  try { const translated = await call<string>('translate_text', { text }); result.textContent = translated; }
  catch (error) { result.textContent = errorMessage(error); result.classList.add('error'); }
  finally { button.disabled = false; }
}
async function refreshModels(button: HTMLButtonElement) {
  const list = requireElement<HTMLDivElement>('#model-list'); button.disabled = true; list.innerHTML = '<p class="muted small">正在获取 OpenRouter 免费模型…</p>';
  try { freeModels = await call<FreeModel[]>('refresh_free_models'); modelsLoaded = true; list.innerHTML = modelList(); }
  catch (error) { list.innerHTML = `<p class="error small">刷新失败：${escapeHtml(errorMessage(error))}</p>`; }
  finally { button.disabled = false; }
}
async function handleMainClick(event: Event) {
  const button = (event.target as HTMLElement).closest<HTMLButtonElement>('button'); if (!button) return;
  const target = button.dataset.page as Page | undefined;
  if (target) { go(target); return; }
  try {
    if (button.dataset.url) { await call('open_url', { url: button.dataset.url }); return; }
    switch (button.dataset.action) {
      case 'to-settings': go('settings'); break;
      case 'to-discover': go('discover'); break;
      case 'to-translate': go('translate'); break;
      case 'use-mymemory': await selectProvider('mymemory'); break;
      case 'use-siliconflow': await selectProvider('siliconflow'); break;
      case 'use-aliyun': await selectProvider('aliyun'); break;
      case 'use-openrouter': await selectProvider('openrouter'); break;
      case 'toggle-key': { const input = requireElement<HTMLInputElement>('#api-key'); input.type = input.type === 'password' ? 'text' : 'password'; button.textContent = input.type === 'password' ? '显示' : '隐藏'; button.setAttribute('aria-label', input.type === 'password' ? '显示密钥' : '隐藏密钥'); break; }
      case 'save-key': await saveKey(); break;
      case 'clear-key': { const provider = settings.provider; await call('clear_api_key', { provider }); delete availableModels[provider]; if (settings.provider === provider) { settings.has_key = false; settings.credential_error = null; modelStatus = ''; renderAtCurrentScroll(); notify('密钥已清除'); } break; }
      case 'save-model': await saveProviderModel(button); break;
      case 'fetch-provider-models': await fetchProviderModels(button); break;
      case 'save-settings': await saveSettings(); break;
      case 'manual-translate': await manualTranslate(button); break;
      case 'refresh-models': await refreshModels(button); break;
    }
  } catch (error) { notify(errorMessage(error), 'error'); }
}

function renderOverlay() {
  document.body.className = 'overlay-body';
  app.innerHTML = `<button id="translate-bubble" class="translate-bubble" aria-label="翻译选中文字" title="翻译选中文字">${icons.translate}</button><div id="toast" class="toast overlay-toast" role="status"></div>`;
  document.querySelector('#translate-bubble')?.addEventListener('click', async () => { try { await call('open_translation'); } catch (error) { notify(errorMessage(error), 'error'); } });
}
function renderTranslation() {
  document.body.className = 'translation-body';
  app.innerHTML = `<div class="translation-window"><header class="translation-header"><div class="translation-brand"><span class="translation-mark">${icons.spark}</span><strong>一划</strong><span class="translation-caption">选中即达</span></div><div class="translation-header-actions"><button class="icon-button" id="open-main" title="打开设置" aria-label="打开设置">${icons.settings}</button><button class="icon-button" id="close-translation" title="关闭" aria-label="关闭">${icons.close}</button></div></header><div class="translation-content"><div class="translation-label"><span>原文</span><span id="source-length"></span></div><p class="source-text" id="source-text">等待选中文字</p><div class="translation-divider"><span>${icons.translate}</span></div><div class="translation-label"><span>译文</span><span id="translation-provider"></span></div><div id="translation-result" class="translation-result" aria-live="polite">等待翻译</div></div><footer class="translation-footer"><span>原文发送至所选服务商</span><div class="translation-actions"><button class="copy-action" id="retry-result" hidden>重试</button><button class="copy-action" id="copy-result" disabled>${icons.copy}<span>复制译文</span></button></div></footer></div><div id="toast" class="toast" role="status"></div>`;
  document.querySelector('#close-translation')?.addEventListener('click', () => { void call('dismiss_overlay').catch(error => notify(errorMessage(error), 'error')); });
  document.querySelector('#open-main')?.addEventListener('click', () => { void call('show_main').catch(error => notify(errorMessage(error), 'error')); });
  document.querySelector('#copy-result')?.addEventListener('click', () => { void call('copy_text', { text: lastResult }).then(() => notify('译文已复制')).catch(error => notify(errorMessage(error), 'error')); });
  document.querySelector('#retry-result')?.addEventListener('click', () => { void translateSelection(true); });
  document.addEventListener('keydown', event => { if (event.key === 'Escape') void call('dismiss_overlay').catch(error => notify(errorMessage(error), 'error')); });
}
async function translateSelection(retry = false) {
  const generation = ++selectionGeneration; lastResult = '';
  if (!retry) lastSource = '';
  const source = requireElement<HTMLParagraphElement>('#source-text');
  const length = requireElement<HTMLSpanElement>('#source-length');
  const result = requireElement<HTMLDivElement>('#translation-result');
  const copy = requireElement<HTMLButtonElement>('#copy-result');
  const retryButton = requireElement<HTMLButtonElement>('#retry-result');
  const provider = requireElement<HTMLSpanElement>('#translation-provider');
  retryButton.hidden = true; provider.textContent = '';
  copy.disabled = true; source.textContent = '正在获取选中文字…'; length.textContent = '';
  result.innerHTML = '<span class="loading-dots"><i></i><i></i><i></i></span> 正在翻译'; result.classList.remove('error');
  try {
    const text = retry ? lastSource : await call<string>('get_pending_selection');
    if (generation !== selectionGeneration) return;
    if (!text.trim()) throw new Error('没有获取到选中文字，请重新选择。');
    lastSource = text;
    source.textContent = text; length.textContent = `${text.length} 字符`;
    try { const current = await call<Settings>('get_settings'); provider.textContent = current.provider === 'mymemory' ? '由 MyMemory 提供' : `由 ${cloudProviders[current.provider as CloudProvider]?.name || current.provider} 提供`; } catch { provider.textContent = ''; }
    if (generation !== selectionGeneration) return;
    const translated = await call<string>('translate_text', { text });
    if (generation !== selectionGeneration) return;
    lastResult = translated; result.textContent = translated; copy.disabled = false;
  } catch (error) {
    if (generation !== selectionGeneration) return;
    if (source.textContent === '正在获取选中文字…') source.textContent = '未获取到文字';
    result.textContent = errorMessage(error); result.classList.add('error');
    retryButton.hidden = !lastSource;
  }
}
async function bootstrap() {
  if (windowKind === 'overlay') { renderOverlay(); return; }
  if (windowKind === 'translation') {
    renderTranslation();
    if (isTauri) { try { await listen<SelectionEvent>('selection', () => { void translateSelection(); }); await call('translation_ready'); } catch (error) { const result = requireElement<HTMLDivElement>('#translation-result'); result.textContent = `无法接收划词：${errorMessage(error)}`; result.classList.add('error'); } }
    else { requireElement<HTMLParagraphElement>('#source-text').textContent = 'A little curiosity goes a long way.'; requireElement<HTMLDivElement>('#translation-result').textContent = '一点好奇心，可以走很远。'; }
    return;
  }
  try { settings = await call<Settings>('get_settings'); } catch (error) { loadError = `无法读取应用状态：${errorMessage(error)}`; }
  renderMain(); app.addEventListener('click', event => { void handleMainClick(event); });
  app.addEventListener('change', event => {
    const select = event.target as HTMLSelectElement;
    if (select.id !== 'provider-select') return;
    const provider = select.value as Provider; select.disabled = true;
    void selectProvider(provider).catch(error => { select.value = settings.provider; select.disabled = false; notify(errorMessage(error), 'error'); });
  });
  app.addEventListener('input', event => { const target = event.target as HTMLInputElement; if (target.id === 'workspace-id') workspaceId = target.value; });
}
void bootstrap();

