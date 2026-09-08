/**
 * store-i18n.js — Internationalization Helper for Storefront
 */
(function () {
  const STORAGE_KEY = 'app_lang';
  let currentLang = localStorage.getItem(STORAGE_KEY) || 'id';
  let dictionaries = { id: null, en: null };

  async function loadDictionary(lang) {
    if (dictionaries[lang]) return dictionaries[lang];
    try {
      const res = await fetch(`/assets/i18n/${lang}.json`);
      if (res.ok) {
        dictionaries[lang] = await res.json();
        return dictionaries[lang];
      }
    } catch (e) {
      console.warn(`[i18n] Failed to fetch /assets/i18n/${lang}.json`, e);
    }
    return null;
  }

  function getNested(obj, path) {
    if (!obj) return undefined;
    const parts = path.split('.');
    let cur = obj;
    for (const p of parts) {
      if (cur && typeof cur === 'object' && p in cur) {
        cur = cur[p];
      } else {
        return undefined;
      }
    }
    return typeof cur === 'string' ? cur : undefined;
  }

  function translate(key, fallback) {
    const dict = dictionaries[currentLang];
    const val = getNested(dict, key);
    if (val !== undefined) return val;

    if (currentLang !== 'id') {
      const idVal = getNested(dictionaries.id, key);
      if (idVal !== undefined) return idVal;
    }

    return fallback !== undefined ? fallback : key;
  }

  function applyTranslations() {
    document.documentElement.lang = currentLang;
    const elements = document.querySelectorAll('[data-i18n]');
    elements.forEach((el) => {
      const key = el.getAttribute('data-i18n');
      if (key) {
        const translated = translate(key, el.textContent);
        el.textContent = translated;
      }
    });

    const placeholders = document.querySelectorAll('[data-i18n-placeholder]');
    placeholders.forEach((el) => {
      const key = el.getAttribute('data-i18n-placeholder');
      if (key) {
        el.placeholder = translate(key, el.placeholder);
      }
    });

    const langIcon = document.getElementById('store-lang-icon');
    if (langIcon) {
      langIcon.textContent = currentLang === 'id' ? '🇮🇩 ID' : '🇬🇧 EN';
    }
  }

  async function setLanguage(lang) {
    currentLang = lang;
    localStorage.setItem(STORAGE_KEY, lang);
    await loadDictionary(lang);
    applyTranslations();
  }

  async function toggleStoreLanguage() {
    const next = currentLang === 'id' ? 'en' : 'id';
    await setLanguage(next);
  }

  // Initialize
  window.I18n = {
    get current() { return currentLang; },
    t: translate,
    setLanguage,
    toggle: toggleStoreLanguage,
    apply: applyTranslations,
  };
  window.toggleStoreLanguage = toggleStoreLanguage;

  document.addEventListener('DOMContentLoaded', async () => {
    await Promise.all([loadDictionary('id'), loadDictionary(currentLang)]);
    applyTranslations();
  });
})();
