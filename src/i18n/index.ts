import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'

export const en = {
  app: {
    name: 'POS Global',
    tagline: 'Enterprise Point of Sale & Retail Management',
    skipToContent: 'Skip to main content',
    breadcrumb: 'Breadcrumbs',
  },
  nav: {
    sections: {
      operations: 'Operations',
      management: 'Management',
      administration: 'Administration',
    },
    items: {
      pos: 'Point of Sale',
      shifts: 'Cash & Register',
      inventory: 'Products & Stock',
      customers: 'Customers & Debt',
      reports: 'Reports & Analytics',
      users: 'Users & Access',
      tenants: 'Org & Branches',
      settings: 'System Settings',
    },
    collapse: 'Collapse sidebar',
    expand: 'Expand sidebar',
  },
  header: {
    organization: 'Organization',
    branch: 'Branch',
    register: 'Register',
    lockSession: 'Lock Terminal',
    logout: 'Logout',
    profile: 'User Profile',
  },
  status: {
    online: 'Online',
    offline: 'Offline',
    syncing: 'Syncing',
    offlineBanner: 'Offline Mode: Transactions and data are stored locally and will sync once connection is restored.',
    pendingChanges_one: '{{count}} pending change queued for synchronization',
    pendingChanges_other: '{{count}} pending changes queued for synchronization',
    currentRegister: 'Terminal: {{name}}',
    systemReady: 'System Ready',
  },
  states: {
    loading: {
      title: 'Loading Application Data',
      description: 'Fetching operational context and initializing state...',
    },
    empty: {
      title: 'No Records Available',
      description: 'There is currently no data to display in this view.',
      action: 'Refresh View',
    },
    error: {
      title: 'Operation Failed',
      description: 'An unexpected error occurred while processing your request.',
      retry: 'Try Again',
      contactSupport: 'Report Issue',
    },
    permissionDenied: {
      title: 'Access Restricted',
      description: 'Your current role or session does not have authorization for this operation (Requires permission: {{permission}}).',
      action: 'Return to Dashboard',
    },
  },
  languages: {
    select: 'Language selection',
    en: 'English',
    ar: 'العربية',
    fr: 'Français',
  },
  shortcuts: {
    pos: 'F1',
    shifts: 'F2',
    lock: 'F9',
  },
} as const

export const ar = {
  app: {
    name: 'POS Global',
    tagline: 'نظام نقاط البيع وإدارة المتاجر العالمي',
    skipToContent: 'الانتقال إلى المحتوى الرئيسي',
    breadcrumb: 'مسار التنقل',
  },
  nav: {
    sections: {
      operations: 'العمليات',
      management: 'الإدارة',
      administration: 'النظام والصلاحيات',
    },
    items: {
      pos: 'نقطة البيع',
      shifts: 'الصندوق والورديات',
      inventory: 'المنتجات والمخزون',
      customers: 'العملاء والديون',
      reports: 'التقارير والتحليلات',
      users: 'المستخدمون والصلاحيات',
      tenants: 'المؤسسة والفروع',
      settings: 'إعدادات النظام',
    },
    collapse: 'طي القائمة الجانبية',
    expand: 'توسيع القائمة الجانبية',
  },
  header: {
    organization: 'المؤسسة',
    branch: 'الفرع',
    register: 'نقطة البيع',
    lockSession: 'قفل الشاشة',
    logout: 'تسجيل الخروج',
    profile: 'الملف الشخصي',
  },
  status: {
    online: 'متصل',
    offline: 'غير متصل',
    syncing: 'جاري المزامنة',
    offlineBanner: 'وضع عدم الاتصال: يتم حفظ العمليات محلياً وستتم المزامنة تلقائياً عند استعادة الاتصال.',
    pendingChanges_zero: 'لا توجد عمليات بانتظار المزامنة',
    pendingChanges_one: 'عملية واحدة بانتظار المزامنة',
    pendingChanges_two: 'عمليتان بانتظار المزامنة',
    pendingChanges_few: '{{count}} عمليات بانتظار المزامنة',
    pendingChanges_many: '{{count}} عملية بانتظار المزامنة',
    pendingChanges_other: '{{count}} عملية بانتظار المزامنة',
    currentRegister: 'الجهاز: {{name}}',
    systemReady: 'النظام جاهز',
  },
  states: {
    loading: {
      title: 'جاري تحميل البيانات',
      description: 'يتم جلب البيانات وتجهيز بيئة العمل...',
    },
    empty: {
      title: 'لا توجد سجلات',
      description: 'لا توجد بيانات متاحة للعرض في هذه الشاشة حالياً.',
      action: 'تحديث العرض',
    },
    error: {
      title: 'فشل تنفيذ العملية',
      description: 'حدث خطأ غير متوقع أثناء معالجة الطلب.',
      retry: 'إعادة المحاولة',
      contactSupport: 'الإبلاغ عن المشكلة',
    },
    permissionDenied: {
      title: 'غير مصرح بالوصول',
      description: 'حسابك الحالي لا يمتلك الصلاحية المطلوبة لتنفيذ هذا الإجراء (الصلاحية المطلوبة: {{permission}}).',
      action: 'العودة للرئيسية',
    },
  },
  languages: {
    select: 'اختيار اللغة',
    en: 'English',
    ar: 'العربية',
    fr: 'Français',
  },
  shortcuts: {
    pos: 'F1',
    shifts: 'F2',
    lock: 'F9',
  },
} as const

export const fr = {
  app: {
    name: 'POS Global',
    tagline: 'Point de Vente & Gestion Commerciale Entreprise',
    skipToContent: 'Passer au contenu principal',
    breadcrumb: "Fil d'Ariane",
  },
  nav: {
    sections: {
      operations: 'Opérations',
      management: 'Gestion',
      administration: 'Administration',
    },
    items: {
      pos: 'Point de Vente',
      shifts: 'Caisse & Sessions',
      inventory: 'Produits & Stock',
      customers: 'Clients & Crédits',
      reports: 'Rapports & Analyses',
      users: 'Utilisateurs & Droits',
      tenants: 'Organisation & Branches',
      settings: 'Paramètres Système',
    },
    collapse: 'Réduire la barre latérale',
    expand: 'Agrandir la barre latérale',
  },
  header: {
    organization: 'Organisation',
    branch: 'Succursale',
    register: 'Caisse',
    lockSession: 'Verrouiller',
    logout: 'Déconnexion',
    profile: 'Profil',
  },
  status: {
    online: 'En ligne',
    offline: 'Hors ligne',
    syncing: 'Synchronisation',
    offlineBanner: 'Mode Hors Ligne: Les opérations sont enregistrées localement et seront synchronisées dès le rétablissement de la connexion.',
    pendingChanges_one: '{{count}} opération en attente de synchronisation',
    pendingChanges_other: '{{count}} opérations en attente de synchronisation',
    currentRegister: 'Terminal: {{name}}',
    systemReady: 'Système prêt',
  },
  states: {
    loading: {
      title: 'Chargement des Données',
      description: 'Récupération du contexte et initialisation du système...',
    },
    empty: {
      title: 'Aucune Donnée Disponible',
      description: "Il n'y a actuellement aucune donnée à afficher pour cette vue.",
      action: 'Actualiser',
    },
    error: {
      title: "Échec de l'Opération",
      description: 'Une erreur inattendue est survenue lors du traitement.',
      retry: 'Réessayer',
      contactSupport: 'Signaler un Problème',
    },
    permissionDenied: {
      title: 'Accès Restreint',
      description: 'Votre rôle ou session actuelle ne dispose pas des autorisations nécessaires (Droit requis: {{permission}}).',
      action: 'Retour au Tableau de Bord',
    },
  },
  languages: {
    select: 'Sélection de la langue',
    en: 'English',
    ar: 'العربية',
    fr: 'Français',
  },
  shortcuts: {
    pos: 'F1',
    shifts: 'F2',
    lock: 'F9',
  },
} as const

export const defaultLocale = 'en'
export const supportedLocales = ['en', 'ar', 'fr'] as const
export type SupportedLocale = (typeof supportedLocales)[number]

export const rtlLocales: ReadonlySet<string> = new Set(['ar'])

export function getDirectionForLocale(locale: string): 'ltr' | 'rtl' {
  return rtlLocales.has(locale) ? 'rtl' : 'ltr'
}

void i18n
  .use(initReactI18next)
  .init({
    resources: {
      en: { translation: en },
      ar: { translation: ar },
      fr: { translation: fr },
    },
    lng: defaultLocale,
    fallbackLng: defaultLocale,
    interpolation: {
      escapeValue: false,
    },
  })

export default i18n
