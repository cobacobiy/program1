import idDict from './i18n/id.json';
import enDict from './i18n/en.json';

export type Language = 'id' | 'en';

const STORAGE_KEY = 'app_lang';

const DICTIONARIES: Record<Language, Record<string, any>> = {
  id: idDict,
  en: enDict,
};

class I18nState {
  current = $state<Language>('id');

  constructor() {
    if (typeof window !== 'undefined') {
      const saved = localStorage.getItem(STORAGE_KEY) as Language | null;
      if (saved && (saved === 'id' || saved === 'en')) {
        this.current = saved;
        document.documentElement.lang = saved;
      } else {
        const browserLang = navigator.language?.toLowerCase();
        if (browserLang && browserLang.startsWith('en')) {
          this.current = 'en';
          document.documentElement.lang = 'en';
        } else {
          this.current = 'id';
          document.documentElement.lang = 'id';
        }
      }
    }
  }

  setLanguage(lang: Language) {
    this.current = lang;
    if (typeof window !== 'undefined') {
      localStorage.setItem(STORAGE_KEY, lang);
      document.documentElement.lang = lang;
    }
  }

  toggleLanguage() {
    this.setLanguage(this.current === 'id' ? 'en' : 'id');
  }

  t(path: string, fallback?: string): string {
    const dict = DICTIONARIES[this.current] || DICTIONARIES.id;
    const parts = path.split('.');
    let val: any = dict;
    for (const part of parts) {
      if (val && typeof val === 'object' && part in val) {
        val = val[part];
      } else {
        val = undefined;
        break;
      }
    }

    if (val !== undefined && typeof val === 'string') {
      return val;
    }

    // Fallback to id dictionary
    if (this.current !== 'id') {
      let fallbackVal: any = DICTIONARIES.id;
      for (const part of parts) {
        if (fallbackVal && typeof fallbackVal === 'object' && part in fallbackVal) {
          fallbackVal = fallbackVal[part];
        } else {
          fallbackVal = undefined;
          break;
        }
      }
      if (fallbackVal !== undefined && typeof fallbackVal === 'string') {
        return fallbackVal;
      }
    }

    return fallback ?? path;
  }
}

export const i18n = new I18nState();
