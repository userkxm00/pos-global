# V2_RULES.md — دستور المشروع لكل AI Agent

**هذا الملف يتقرا وجوباً فـ بداية كل session، قبل أي سطر كود.**
مكتوب بعد تجربة حقيقية مع مشروع سابق (Mellah POS V2) وقع فيه AI agent فـ أخطاء أمنية ومالية خطيرة، وكان يدعي "الاكتمال" بلا دليل. هذا المشروع الجديد — POS Global — ما يتبناش نفس الأخطاء.

خدمة وحيدة هنا (Karim) وحده مع agent. ما كاينش مراجعة زميل ثانية. **الـ agent هو خط الدفاع الأول، وهذا الملف هو الحدود اللي ما يتعداهاش.**

## 1. مين يقرر شنو — حدود سلطة الـ Agent

الـ agent يقدر يقرر بروحو فـ: تسمية المتغيرات، تنظيم الملفات الثانوية، كتابة تعليقات، refactoring بسيط بلا تغيير سلوك.

الـ agent **ما يقدرش** يقرر بروحو، ولازم يسأل أو يوقف وينتظر تأكيد صريح فـ:
- أي تعديل على `src-tauri/src/licence/` (منطق الترخيص)
- أي تعديل على منطق العمليات المالية (`sales.rs`, `db/` المرتبطة بالمعاملات)
- إضافة أو حذف dependency جديدة (فـ `Cargo.toml` أو `package.json`)
- تغيير schema قاعدة البيانات (migration جديدة)
- أي قرار معماري (architecture) ما هو مذكور فـ `ARCHITECTURE.md`

## 2. القواعد الصارمة (لا نقاش فيها)

### 2.1 — لا وصول مباشر من الواجهة
الواجهة (React/TS) **ما توصلش** مباشرة لقاعدة البيانات، الملفات، أو أي مورد نظام. كل تفاعل يمر عبر **Tauri command** موجود ومسجل فـ `src-tauri/src/commands/`.
> *السبب*: فـ V2، 35 ملف كانو يهدرو مباشرة عبر IPC غير محمي — هذا فتح باب لثغرات أمنية متعددة.

### 2.2 — التحقق من الصلاحيات فـ Rust فقط
التحقق من PIN، الأدوار (roles)، والصلاحيات يتم **حصرياً فـ Rust backend**. الواجهة تعرض النتيجة فقط، **ما تقررش أبداً**.
> *السبب*: ثغرة PIN bypass كانت فـ `SessionLockModal` (JS) فـ V2 — كان يمكن تجاوزها من جهة الواجهة.

### 2.3 — العمليات المالية atomic إجبارياً
أي عملية بيع، تسديد دين، استرجاع، أو تعديل مخزون مرتبط بمعاملة مالية، تتم داخل **transaction واحدة atomic** فـ SQLite. حقول إجبارية (`shift_id`, `user_id`) **ما تتقبلش أبداً بقيمة فارغة أو NULL**.
> *السبب*: `shift_id = NULL` فـ تسديد الديون كان bug مالي حقيقي فـ V2 — عمليات "نص محفوظة" بلا مسؤول واضح عليها.

### 2.4 — منطق الترخيص معزول ومحمي
كل شي متعلق بالترخيص (`src-tauri/src/licence/`) يبقى معزول فـ module خاص بيه. **ممنوع** يتكتب أي جزء منه مباشرة فـ `commands/` أو أي مكان آخر. الحالة المحلية للترخيص لازم تكون **موقعة رقمياً (signed)** — المستخدم ما يقدرش يعدلها يدوياً.

### 2.5 — لا نص مباشر فـ الواجهة (i18n إجباري)
كل نص يظهر للمستخدم يمر عبر مفاتيح الترجمة فـ `src/i18n/` (`ar.json`, `fr.json`, `en.json`). **ممنوع** نص مكتوب مباشرة (hardcoded) داخل component.

### 2.6 — لا SQL injection
كل استعلام SQL يستعمل **prepared statements** (parameterized queries). **ممنوع نهائياً** بناء query بـ string concatenation.

## 3. قاعدة "المكتمل" — الأهم فـ كامل هذا الملف

> **Feature ماشي "مكتملة" إلا إذا عندها اختبار حقيقي (test) يعدي، وأنت شفت نتيجته بعينيك.**

- ممنوع تماماً الجمل زي: "كل شيء يخدم 100%"، "خلصت من كل شيء"، "ما فماش مشاكل" — بلا دليل قابل للتحقق (output حقيقي من `cargo test` أو من frontend test runner).
- إذا مقدرتش تكتب test لسبب معين (مثلاً يحتاج UI حقيقي)، قلها بصراحة: "هذا الجزء ما عندوش test أوتوماتيكي، خاصك تجربو يدوياً" — بلا ما تدعي الاكتمال.
- قبل ما تقول "خلصت feature X"، اعرض:
  1. شنو تبدل بالضبط (ملفات + سطور)
  2. نتيجة الاختبار (test output الفعلي)
  3. أي حاجة ما تديتش تختبرها بعد

**هذا كان بالضبط السبب اللي خلاني (Claude) نكتشف فـ audit سابق أن agent كان يقول "100% مكتمل" وهو كاذب. ما نعاودوهاش هنا.**

## 4. سير العمل (Workflow)

- **Commits صغيرة ومتكررة** — كل commit يمثل تغيير واحد منطقي، برسالة واضحة (شنو تبدل + علاش)
- قبل بداية أي feature جديدة: اقرا هذا الملف + `ARCHITECTURE.md`
- إذا لقيت نمط (pattern) مكرر فـ مكان آخر من الكود، **اسأل قبل ما تكتب نسخة جديدة** — يمكن كاين حل موحد أحسن
- لا تبدل ملف migration تم تطبيقه بالفعل — migration جديدة دايماً، أبداً تعديل قديمة

## 5. أخطاء V2 — مرجع سريع (لا تعاود أي وحدة منهم)

| الخطأ فـ V2 | الحل هنا |
|---|---|
| 35 ملف IPC مباشر بلا حماية | Command pattern إجباري (قاعدة 2.1) |
| PIN bypass فـ `SessionLockModal` (JS) | التحقق فـ Rust فقط (قاعدة 2.2) |
| `shift_id = NULL` فـ تسديد الديون | حقل إجباري + transaction atomic (قاعدة 2.3) |
| agent يدعي "100% مكتمل" بلا دليل | قاعدة الاكتمال الإجبارية (قسم 3) |
| نصوص متفرقة بلغة وحدة فـ الكود | i18n إجباري من البداية (قاعدة 2.5) |

## 6. لما تكون غير متأكد

إذا الطلب غامض، أو فيه أكثر من طريقة تنفيذ ممكنة، أو يمس واحد من الأقسام الحساسة (§1):
**توقف واسأل، ماشي تخمن وتكمل.** سؤال واحد واضح أحسن من ساعات تصليح بعد.

# 7. Global Commerce Rules

## 7.1 — Industry is not Product Type

Never add:

```text
product_type = clothing
product_type = grocery
product_type = electronics
...
```

to solve industry expansion.

Use:

```text
industry preset
+
capabilities
+
domain module
```

instead.

## 7.2 — Mixed Stores Are First-Class

A general store may contain:

- clothing with matrix
- food with weight/batch/expiry
- electronics with serial/IMEI/warranty
- furniture with dimensions/material

in the same database and branch.

## 7.3 — JSON Is an Escape Hatch

`custom_attributes` is allowed only for exceptional metadata.

Never move core money, stock, tax, serial, batch or payment data into JSON simply to avoid schema design.

## 7.4 — Financial Precision

`f64`/floating-point is forbidden for production financial truth.

The current prototype fields using `REAL/f64` are transitional and must be migrated before production.

## 7.5 — Stock Ledger

Every inventory change must have a traceable stock movement.

A direct quantity update without a movement is a defect unless it is part of the atomic operation that creates that movement.

## 7.6 — Domain Modules

Restaurant, Service, Rental, Hospitality and Wholesale workflows must remain modular.

Do not turn every module into a product-type branch inside core sales code.

## 7.7 — Idempotency

Every retryable financial/sync operation must be safe against duplicate execution.

## 7.8 — Applied Migrations

Never edit an applied migration. Add a new migration.

## 7.9 — Dependencies

Any dependency addition must include:

- why it is required
- alternatives considered
- security/license considerations
- impact on build/platform support
- tests

## 7.10 — Naming

A proposed brand must not be added to package IDs, bundle IDs, URLs or license infrastructure until it passes brand/domain/trademark screening.
