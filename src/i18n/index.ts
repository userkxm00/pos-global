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
  onboarding: {
    title: 'Initial Setup Wizard',
    subtitle: 'Configure your organization, primary branch, and point of sale terminal to get started.',
    steps: {
      organization: 'Organization',
      branch: 'Branch',
      register: 'Register & Device',
      complete: 'Complete',
    },
    org: {
      title: 'Create Your Organization',
      description: 'Set up your business identity, default accounting currency, and operating language.',
      nameLabel: 'Organization Name',
      namePlaceholder: 'e.g. Acme Retail International',
      currencyLabel: 'Default Currency',
      languageLabel: 'Default Language',
      currencyHint: 'Authoritative ISO 3-letter currency code for financial transactions.',
      languageHint: 'Primary interface language for receipts and administrative reporting.',
    },
    branch: {
      title: 'Configure Primary Branch',
      description: 'Define your main store, warehouse, or retail point of sale branch.',
      nameLabel: 'Branch Name',
      namePlaceholder: 'e.g. Downtown Flagship Store',
      addressLabel: 'Address / Physical Location',
      addressPlaceholder: 'e.g. 123 Main Street, Suite 400',
      currencyLabel: 'Branch Currency',
      currencyHint: 'Transactions in this branch will default to this currency.',
    },
    register: {
      title: 'Register & Point of Sale Terminal',
      description: 'Set up the active cash register or checkout terminal on this device.',
      nameLabel: 'Register / Terminal Name',
      namePlaceholder: 'e.g. POS-01 (Front Counter)',
      codeLabel: 'Device / Terminal Code',
      codePlaceholder: 'e.g. REG-MAIN-01',
      codeHint: 'Optional unique identifier for register hardware audits.',
    },
    complete: {
      title: 'System Initialization Complete!',
      description: 'Your point of sale environment is successfully initialized and ready for operations.',
      summaryTitle: 'Configuration Summary',
      orgLabel: 'Organization:',
      branchLabel: 'Branch:',
      registerLabel: 'Active Terminal:',
      currencyLabel: 'Currency:',
      launchAction: 'Enter POS Global',
    },
    actions: {
      next: 'Next Step',
      back: 'Back',
      createOrg: 'Create Organization',
      createBranch: 'Create Branch',
      createRegister: 'Finish Setup',
      submitting: 'Saving Configuration...',
      retry: 'Retry Submission',
    },
    validation: {
      orgNameRequired: 'Organization name is required.',
      orgNameTooLong: 'Organization name must not exceed 255 characters.',
      currencyInvalid: 'Currency must be a valid 3-letter uppercase ISO code.',
      languageInvalid: 'Language code must be between 2 and 10 characters.',
      missingOrgContext: 'Organization context is missing. Please create an organization first.',
      branchNameRequired: 'Branch name is required.',
      branchNameTooLong: 'Branch name must not exceed 255 characters.',
      addressTooLong: 'Address must not exceed 500 characters.',
      missingBranchContext: 'Branch context is missing. Please configure a branch first.',
      registerNameRequired: 'Register name is required.',
      registerNameTooLong: 'Register name must not exceed 255 characters.',
      registerCodeTooLong: 'Register code must not exceed 50 characters.',
    },
    errors: {
      databaseGeneric: 'A database error occurred while processing the request. Please try again.',
      unknown: 'An unexpected error occurred during onboarding setup.',
      orgAlreadyExists: 'An organization with this name already exists.',
      orgNotFound: 'The specified organization was not found.',
      branchNotFound: 'The specified branch was not found.',
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
      shifts: 'الورديات والخزينة',
      inventory: 'المنتجات والمخزون',
      customers: 'العملاء والديون',
      reports: 'التقارير والتحليلات',
      users: 'المستخدمين والصلاحيات',
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
    lockSession: 'قفل الجهاز',
    logout: 'تسجيل الخروج',
    profile: 'الملف الشخصي',
  },
  status: {
    online: 'متصل',
    offline: 'غير متصل',
    syncing: 'جاري المزامنة',
    offlineBanner: 'وضع عدم الاتصال: يتم حفظ العمليات محلياً وستتم المزامنة تلقائياً عند استعادة الاتصال.',
    pendingChanges_zero: 'لا توجد تغييرات معلقة للمزامنة',
    pendingChanges_one: 'تغيير واحد قيد المزامنة',
    pendingChanges_two: 'تغييران قيد المزامنة',
    pendingChanges_few: '{{count}} تغييرات قيد المزامنة',
    pendingChanges_many: '{{count}} تغييراً قيد المزامنة',
    pendingChanges_other: '{{count}} تغيير قيد المزامنة',
    currentRegister: 'الجهاز: {{name}}',
    systemReady: 'النظام جاهز',
  },
  states: {
    loading: {
      title: 'جاري تحميل بيانات النظام',
      description: 'يتم جلب البيانات التشغيلية وتهيئة الجلسة...',
    },
    empty: {
      title: 'لا توجد بيانات متاحة',
      description: 'لا توجد عناصر لعرضها في هذه الشاشة حالياً.',
      action: 'تحديث الشاشة',
    },
    error: {
      title: 'فشلت العملية',
      description: 'حدث خطأ غير متوقع أثناء معالجة الطلب.',
      retry: 'إعادة المحاولة',
      contactSupport: 'إبلاغ عن مشكلة',
    },
    permissionDenied: {
      title: 'غير مصرح بالوصول',
      description: 'حسابك أو جلستك الحالية لا تملك الصلاحية الكافية لإتمام هذه العملية (يتطلب صلاحية: {{permission}}).',
      action: 'العودة للوحة التحكم',
    },
  },
  onboarding: {
    title: 'معالج التهيئة الأولى للنظام',
    subtitle: 'قم بإعداد بيانات مؤسستك، الفرع الرئيسي، ونقطة البيع للبدء في استخدام النظام.',
    steps: {
      organization: 'المؤسسة',
      branch: 'الفرع',
      register: 'نقطة البيع والجهاز',
      complete: 'اكتمال التهيئة',
    },
    org: {
      title: 'إنشاء المؤسسة',
      description: 'قم بضبط الهوية التجارية، العملة المحاسبية الأساسية، ولغة النظام الافتراضية.',
      nameLabel: 'اسم المؤسسة',
      namePlaceholder: 'مثال: شركة التجارة العالمية',
      currencyLabel: 'العملة الأساسية',
      languageLabel: 'اللغة الافتراضية',
      currencyHint: 'رمز العملة القياسي المكون من 3 أحرف للعمليات المالية.',
      languageHint: 'اللغة الأساسية لواجهة الاستخدام والإيصالات والتقارير.',
    },
    branch: {
      title: 'تهيئة الفرع الرئيسي',
      description: 'حدد بيانات المتجر الرئيسي أو نقطة البيع الأساسية التابعة للمؤسسة.',
      nameLabel: 'اسم الفرع',
      namePlaceholder: 'مثال: فرع وسط المدينة الرئيسي',
      addressLabel: 'العنوان / الموقع الجغرافي',
      addressPlaceholder: 'مثال: شارع الملك فهد، مبنى 102',
      currencyLabel: 'عملة الفرع',
      currencyHint: 'المعاملات المالية داخل هذا الفرع ستعتمد هذه العملة.',
    },
    register: {
      title: 'نقطة البيع والجهاز',
      description: 'قم بإعداد محطة الدفع أو صندوق الكاشير النشط على هذا الجهاز.',
      nameLabel: 'اسم نقطة البيع / الكاشير',
      namePlaceholder: 'مثال: كاشير 1 (الاستقبال)',
      codeLabel: 'رمز الجهاز / نقطة البيع',
      codePlaceholder: 'مثال: REG-MAIN-01',
      codeHint: 'رمز تعريفي اختياري لتدقيق الأجهزة وعمليات الجرد.',
    },
    complete: {
      title: 'اكتملت تهيئة النظام بنجاح!',
      description: 'تم إعداد بيئة العمل ونقاط البيع بنجاح، والنظام جاهز الآن لبدء العمليات اليومية.',
      summaryTitle: 'ملخص الإعدادات',
      orgLabel: 'المؤسسة:',
      branchLabel: 'الفرع:',
      registerLabel: 'نقطة البيع الحالية:',
      currencyLabel: 'العملة:',
      launchAction: 'بدء تشغيل النظام',
    },
    actions: {
      next: 'الخطوة التالية',
      back: 'رجوع',
      createOrg: 'إنشاء المؤسسة',
      createBranch: 'إنشاء الفرع',
      createRegister: 'إتمام التهيئة',
      submitting: 'جاري حفظ الإعدادات...',
      retry: 'إعادة المحاولة',
    },
    validation: {
      orgNameRequired: 'اسم المؤسسة مطلوب.',
      orgNameTooLong: 'يجب ألا يتجاوز اسم المؤسسة 255 حرفاً.',
      currencyInvalid: 'يجب أن يكون رمز العملة صالحاً ومكوناً من 3 أحرف كبيرة.',
      languageInvalid: 'يجب أن يكون رمز اللغة بين حرفين و 10 أحرف.',
      missingOrgContext: 'معرّف المؤسسة غير موجود. يرجى إنشاء المؤسسة أولاً.',
      branchNameRequired: 'اسم الفرع مطلوب.',
      branchNameTooLong: 'يجب ألا يتجاوز اسم الفرع 255 حرفاً.',
      addressTooLong: 'يجب ألا يتجاوز العنوان 500 حرف.',
      missingBranchContext: 'معرّف الفرع غير موجود. يرجى إعداد الفرع أولاً.',
      registerNameRequired: 'اسم نقطة البيع مطلوب.',
      registerNameTooLong: 'يجب ألا يتجاوز اسم نقطة البيع 255 حرفاً.',
      registerCodeTooLong: 'يجب ألا يتجاوز رمز الجهاز 50 حرفاً.',
    },
    errors: {
      databaseGeneric: 'حدث خطأ في قاعدة البيانات أثناء المعالجة. يرجى المحاولة مرة أخرى.',
      unknown: 'حدث خطأ غير متوقع أثناء عملية التهيئة.',
      orgAlreadyExists: 'توجد مؤسسة بهذا الاسم بالفعل.',
      orgNotFound: 'المؤسسة المحددة غير موجودة.',
      branchNotFound: 'الفرع المحدد غير موجود.',
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
    breadcrumb: 'Fil d’Ariane',
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
      inventory: 'Articles & Stocks',
      customers: 'Clients & Créances',
      reports: 'Rapports & Statistiques',
      users: 'Utilisateurs & Droits',
      tenants: 'Organisation & Succursales',
      settings: 'Paramètres Système',
    },
    collapse: 'Réduire le menu',
    expand: 'Développer le menu',
  },
  header: {
    organization: 'Organisation',
    branch: 'Succursale',
    register: 'Caisse',
    lockSession: 'Verrouiller le terminal',
    logout: 'Déconnexion',
    profile: 'Profil utilisateur',
  },
  status: {
    online: 'En ligne',
    offline: 'Hors ligne',
    syncing: 'Synchronisation...',
    offlineBanner: 'Mode hors ligne : les données sont enregistrées localement et seront synchronisées dès le rétablissement du réseau.',
    pendingChanges_one: '{{count}} modification en attente de synchronisation',
    pendingChanges_other: '{{count}} modifications en attente de synchronisation',
    currentRegister: 'Caisse : {{name}}',
    systemReady: 'Système prêt',
  },
  states: {
    loading: {
      title: 'Chargement des données',
      description: 'Récupération du contexte opérationnel et initialisation...',
    },
    empty: {
      title: 'Aucun enregistrement disponible',
      description: 'Aucune donnée à afficher pour le moment.',
      action: 'Actualiser',
    },
    error: {
      title: 'Échec de l’opération',
      description: 'Une erreur inattendue est survenue lors du traitement.',
      retry: 'Réessayer',
      contactSupport: 'Signaler un problème',
    },
    permissionDenied: {
      title: 'Accès restreint',
      description: 'Votre rôle ou session actuel ne dispose pas des droits requis (Permission requise : {{permission}}).',
      action: 'Retour au tableau de bord',
    },
  },
  onboarding: {
    title: 'Assistant de Première Configuration',
    subtitle: 'Configurez votre organisation, succursale principale et terminal de caisse pour démarrer.',
    steps: {
      organization: 'Organisation',
      branch: 'Succursale',
      register: 'Caisse & Terminal',
      complete: 'Terminé',
    },
    org: {
      title: 'Créer Votre Organisation',
      description: 'Définissez votre identité commerciale, devise comptable principale et langue d’utilisation.',
      nameLabel: 'Nom de l’organisation',
      namePlaceholder: 'ex. Acme Commerce International',
      currencyLabel: 'Devise principale',
      languageLabel: 'Langue par défaut',
      currencyHint: 'Code ISO officiel à 3 lettres pour les transactions financières.',
      languageHint: 'Langue principale pour l’interface, les reçus et les rapports.',
    },
    branch: {
      title: 'Configurer la Succursale Principale',
      description: 'Définissez votre magasin principal, entrepôt ou point de vente.',
      nameLabel: 'Nom de la succursale',
      namePlaceholder: 'ex. Succursale Centre-Ville',
      addressLabel: 'Adresse / Emplacement',
      addressPlaceholder: 'ex. 123 Rue Principale, Suite 400',
      currencyLabel: 'Devise de la succursale',
      currencyHint: 'Les transactions dans cette succursale utiliseront cette devise par défaut.',
    },
    register: {
      title: 'Caisse & Terminal de Vente',
      description: 'Configurez le poste d’encaissement actif sur cet appareil.',
      nameLabel: 'Nom de la caisse / terminal',
      namePlaceholder: 'ex. Caisse 01 (Comptoir)',
      codeLabel: 'Code terminal / matériel',
      codePlaceholder: 'ex. REG-CENTRE-01',
      codeHint: 'Identifiant optionnel pour l’audit des équipements de caisse.',
    },
    complete: {
      title: 'Configuration Initiale Terminée !',
      description: 'Votre environnement de point de vente est opérationnel et prêt pour les ventes.',
      summaryTitle: 'Récapitulatif de Configuration',
      orgLabel: 'Organisation :',
      branchLabel: 'Succursale :',
      registerLabel: 'Terminal actif :',
      currencyLabel: 'Devise :',
      launchAction: 'Accéder à POS Global',
    },
    actions: {
      next: 'Étape Suivante',
      back: 'Retour',
      createOrg: "Créer l'Organisation",
      createBranch: 'Créer la Succursale',
      createRegister: 'Terminer la Configuration',
      submitting: 'Enregistrement...',
      retry: 'Réessayer',
    },
    validation: {
      orgNameRequired: "Le nom de l'organisation est obligatoire.",
      orgNameTooLong: "Le nom de l'organisation ne doit pas dépasser 255 caractères.",
      currencyInvalid: 'La devise doit être un code ISO valide à 3 lettres majuscules.',
      languageInvalid: 'Le code de langue doit comporter entre 2 et 10 caractères.',
      missingOrgContext: "Contexte d'organisation manquant. Veuillez d'abord créer une organisation.",
      branchNameRequired: 'Le nom de la succursale est obligatoire.',
      branchNameTooLong: 'Le nom de la succursale ne doit pas dépasser 255 caractères.',
      addressTooLong: "L'adresse ne doit pas dépasser 500 caractères.",
      missingBranchContext: "Contexte de succursale manquant. Veuillez d'abord créer une succursale.",
      registerNameRequired: 'Le nom de la caisse est obligatoire.',
      registerNameTooLong: 'Le nom de la caisse ne doit pas dépasser 255 caractères.',
      registerCodeTooLong: "Le code de l'appareil ne doit pas dépasser 50 caractères.",
    },
    errors: {
      databaseGeneric: 'Une erreur de base de données est survenue. Veuillez réessayer.',
      unknown: 'Une erreur inattendue est survenue lors de la configuration.',
      orgAlreadyExists: 'Une organisation portant ce nom existe déjà.',
      orgNotFound: "L'organisation spécifiée est introuvable.",
      branchNotFound: 'La succursale spécifiée est introuvable.',
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

export const supportedLocales = ['en', 'ar', 'fr'] as const

export const resources = {
  en: { translation: en },
  ar: { translation: ar },
  fr: { translation: fr },
} as const

export type SupportedLocale = keyof typeof resources

export const defaultLocale: SupportedLocale = 'en'

export function getDirectionForLocale(locale: string): 'rtl' | 'ltr' {
  if (!locale) return 'ltr'
  const normalized = locale.trim().toLowerCase()
  if (normalized === 'ar' || normalized.startsWith('ar-') || normalized.startsWith('ar_')) {
    return 'rtl'
  }
  return 'ltr'
}

void i18n
  .use(initReactI18next)
  .init({
    resources,
    lng: 'en',
    fallbackLng: 'en',
    interpolation: {
      escapeValue: false,
    },
  })

export default i18n
