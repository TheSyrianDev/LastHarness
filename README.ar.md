<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset=".github/assets/lastai-lockup-on-dark.svg">
    <img alt="Last AI" src=".github/assets/lastai-lockup-on-light.svg" width="240">
  </picture>
</p>

<p align="center">
  <a href="README.md">English</a> · <a href="README.ar.md">العربية</a> · <a href="README.zh-CN.md">简体中文</a> · <a href="README.fr.md">Français</a> · <a href="README.de.md">Deutsch</a>
</p>

<p align="center"><em>الإطار الأخير الذي ستحتاجه.</em><br>
إطار ذكاء اصطناعي محلي أولاً، مبني حول فريق: أقسام، والعاملين فيها.</p>

> [!WARNING]
> **ألفا.** Last AI برنامج في مراحله الأولى، ويُستخدم حتى الآن على جهاز واحد فقط. توقع خشونة في التجربة وتغييرات تكسر التوافق. Last AI (التطبيق المكتبي وتطبيق الطرفية `la`) مُختبر على Linux (Omarchy) وWindows (اختبره المؤلف). نسخة macOS في مرحلة ألفا مبكرة: تبدأ في CI، لكن لم يستخدمها أحد يومياً بعد.

<p align="center">
  <a href="https://github.com/TheSyrianDev/LastAI/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/TheSyrianDev/LastAI/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="License: PolyForm Noncommercial 1.0.0" src="https://img.shields.io/badge/license-PolyForm--Noncommercial--1.0.0-blue"></a>
  <img alt="Built with Rust" src="https://img.shields.io/badge/built%20with-Rust-orange">
</p>

Last AI هو إطار ذكاء اصطناعي محلي أولاً لحاسوب شخص واحد. يحفظ البرنامج الأساسي (`lastaid`) المحادثة والفريق (الأقسام والعاملون فيها) والأدوات والذاكرة والتعلم، وهو مستقل عن النموذج، دون أي ارتباط بمزود واحد. يعمل مع أي نموذج أو مزود: خوادم محلية مثل llama-swap وllama.cpp وOllama وLM Studio، أو أي واجهة سحابية متوافقة مع OpenAI أو بأسلوب Anthropic (جُرّب حتى الآن مع llama.cpp عبر llama-swap). الواجهات الأمامية عملاء خفيفون لواجهة برمجة التطبيقات المحلية للبرنامج الأساسي: تطبيق طرفية، وصفحة متصفح، وقنوات مراسلة.
يتذكر عبر الجلسات (ملاحظات يمكنك قراءتها وتحريرها بصيغة Markdown)، ويتعلم عاداتك كقواعد توافق عليها، ويمكنه تسليم المهام الكبيرة إلى أقسام من العاملين بأدوار محددة تخطط وتنفذ وتتحقق من العمل.
الملاحظات والقواعد الحساسة تُستخدم فقط مع نموذج على هذا الجهاز. الصلاحيات متدرجة: القراءة مسموحة دائماً، والكتابة تُطلب مرة واحدة، وكل ما هو خطر يُطلب دائماً.
يعمل على Linux وWindows؛ نسخة macOS في مرحلة ألفا مبكرة.

## البناء

```
cargo build --release
```

يُنتج هذا أربعة برامج في `target/release/`: `lastaid` (البرنامج الأساسي)، و`la` و`lastai` (العميل الطرفي نفسه باسمين مختلفين)، و`lastai-gui` (التطبيق المكتبي). يستغرق التطبيق المكتبي بضع دقائق في البناء لأول مرة؛ أما `cargo build --release -p lh-core -p lh-tui` فيبني البرنامج الأساسي والطرفية فقط.

## أول تشغيل

```
target/release/la setup        # twelve plain questions; Enter accepts each default
target/release/lhd             # start the core (it reads the settings file)
target/release/la              # chat
target/release/lastai-gui # or the desktop app (needs the core running)
```

يجد التطبيق المكتبي البرنامج الأساسي ورمزه بنفسه (الخيارات: `--base` و`--token-file`). له مظهر افتراضي بسيط يمكنك تغييره: ضع ملف `.toml` في `~/.config/lastai/themes/` (السمات الست المدمجة، ولكل منها ثلاث عائلات بنسخة فاتحة وداكنة، موجودة في `crates/lh-gui/themes/` كأمثلة). اضغط Ctrl+N لبدء محادثة جديدة.

نسخة Omarchy (`LastAI-<version>-omarchy-x86_64.tar.gz`، وتُثبَّت بالملف `install-omarchy.sh` الموجود فيها) تقدم أيضاً مظهر سطح المكتب الخاص بها، ومظهر "Omarchy (transparent)" وهو زجاج سائل بألوان Omarchy. يكون المظهر الشفاف بقدر الشفافية التي تحددها قاعدة نافذة Hyprland لنافذة التطبيق (الصنف `lastai-gui`)؛ بدون قاعدة يبقى معتماً. الإصدارات الأخرى (AppImage أو Windows أو macOS أو بناء من المصدر بخيار `--no-default-features`) لا تتضمن مظهري Omarchy. لإضافة أحدهما، ضع هذا السطر في `~/.config/hypr/hyprland.lua` (القيمة 0.85 هي نسبة ظهور النافذة):

```lua
o.window("lastai-gui", { opacity = "0.85 override 0.85 override" })
```

لتشغيل البرنامج الأساسي عند كل تسجيل دخول: `lh service preview` ثم `lh service install` (خدمة مستخدم في systemd؛ لا تحتاج sudo؛ إغلاق الطرفية لا يوقف البرنامج الأساسي أبداً).

## المزودون والنماذج

المزودون هم الخوادم التي تجيب على المحادثة: llama-swap أو llama.cpp المحلي، أو Ollama، أو LM Studio، أو واجهة سحابية متوافقة مع OpenAI أو بأسلوب Anthropic. أضفهم واختبرهم في صفحة النماذج والمفاتيح في التطبيق المكتبي، أو باستخدام `lh settings`. تُقرأ مفاتيح الواجهات من متغيرات بيئة تسميها أنت؛ لا يُرسل أي شيء إلى مزود لم تضفه.

في صفحة النماذج والمفاتيح، ابحث في Hugging Face عن نماذج GGUF. تُجمَّع النتائج حسب مناسبتها لذاكرة هذا الحاسوب (تناسب الذاكرة، ضيقة، أكبر من اللازم، حجم غير معروف). نزّل الملف إلى `~/AI/Models/`. يمكن تسجيل نموذج مُنزَّل لا يخدمه موجه llama-swap بعد بزر Run الخاص به، وهو يحفظ نسخة احتياطية من ملفات الموجه ويتحقق من المصفوفة ويعيد تشغيل llama-swap. النماذج المخدومة بالفعل تعرض "Use in chat". البحث يحتاج إلى الشبكة؛ وبدونها تقول الصفحة ذلك.

## Create (الصور والفيديو)

تتيح لوحة Create في صفحة النماذج والمفاتيح إنشاء صور (Z-Image Turbo) وفيديوهات قصيرة (Wan 2.2 TI2V-5B، بصيغة MP4) عبر ComfyUI محلي على `127.0.0.1:8188`. اكتب وصفاً، واختر Image أو Video، واضغط Generate. تعرض قائمة المهام التقدم، وعند الانتهاء تظهر النتيجة مباشرة (الصورة، أو الإطار الأول من الفيديو). تُحفظ الملفات في `~/AI/Create/`. تأخذ مهام الصور والفيديو قفل الجهاز وتحرر ذاكرة الرسوميات بعدها، لذلك تنتظر عندما تمسك جلسة أخرى بالقفل. ComfyUI وملفات نماذجه ليست جزءاً من هذا المستودع.

## البحث على الويب

يستخدم `web_search` خادم SearXNG. إذا لم تسمِّ الإعدادات أي خادم، يبحث Last AI عن واحد على `127.0.0.1:8888`. إذا لم يرد أحد هناك، تخبر الأداة بذلك وتقترح تشغيل SearXNG أو تحديد عنوانه.

## التثبيت وبناء الإصدار

- من المصدر: `cargo build --release` (يحتاج Rust المستقر؛ على Linux يحتاج التطبيق المكتبي مكتبات GPUI والمكتبات النظامية المعتادة).
- AppImage على Linux: `packaging/build-appimage.sh` (يحتاج `linuxdeploy`؛ راجع رأس السكربت).
- Omarchy: `scripts/install-omarchy.sh`.

## الأوامر

كل أمر لديه `help`، ويعمل دون تشغيل البرنامج الأساسي، مثل `lh team help`.

- `lastaid` هو البرنامج الأساسي. يستمع على `127.0.0.1:7842` فقط. تأتي الإعدادات من `config.toml`؛ وخيار سطر الأوامر يتجاوز الملف. الرمز الذي تحتاجه العملاء يُكتب في `$XDG_RUNTIME_DIR/lastai/token`.
- `la` و`lastai` يفتحان محادثة الطرفية. الخيارات: `--url` و`--token-file` و`--model` و`--ask-all` و`--incognito`.
- `lh setup`: أسئلة التشغيل الأول. `lh settings`: عرض كل إعداد وتغييره (`set` و`reset`).
- `lh memory ...`: الملاحظات التي يحتفظ بها المساعد. `lh learn ...`: القواعد والأسلوب والمهارات والاقتراحات التي تنتظر موافقتك.
- `lh team ...`: الأدوار وأنواع النماذج والأقسام، وإعطاء هدف لقسم (`lh team run`)، والعاملون، ووضع الغياب، ومجلدات الإضافات.
- `lh mcp ...` و`lh trust <folder>`: خوادم MCP (لا يعمل شيء قبل موافقتك) وأي مجلدات مشروع مسموح لها بجلب إعداداتها الخاصة.
- `lh devices ...`: إقران المتصفحات (`lh devices pair` ثم `lh devices accept <PIN>`)، وعرضها وإزالتها.
- `lh channels ...`: قنوات Telegram وMatrix وSlack وSignal (كلها مغلقة حتى تفعّلها).
- `lh export <folder>` و`lh import <folder>`: نقل الذاكرة والقواعد والأسلوب والأدوار والأقسام.
- `lh service ...`: تشغيل البرنامج الأساسي عند تسجيل الدخول.

تُقدَّم صفحة المتصفح عبر HTTPS عند ضبط `web.listen` (مثلاً `lh settings set web.listen 127.0.0.1:7843`)؛ ويقترن المتصفح برمز PIN أولاً.

## أين تُحفظ الملفات

- الإعدادات: `~/.config/lastai/config.toml` (وأسعار الإنفاق الاختيارية في `spend.toml` بجانبه).
- البيانات: `~/.local/share/lastai/`: `lh.db` (سجل النشاط والقواعد والفهارس)، و`memory/` (ملاحظاتك بصيغة Markdown)، و`skills/`، و`team/` (الأدوار والأقسام)، و`mcp.json`، و`web/` (شهادة بوابة المتصفح).
- الرمز: `$XDG_RUNTIME_DIR/lastai/token`.

## لم يُبن بعد

- تطبيق للهاتف، وأي تصميم بصري نهائي (لا شعار ولا تنسيق للاسم ولا ألوان علامة بعد). صفحة المتصفح ومظهر التطبيق المكتبي الافتراضي بسيطان عن قصد.
- macOS؛ برامج التثبيت؛ التحديثات الموقعة؛ تقارير الأعطال مُجهزة فقط ولا تُرسل.
- الخدمة السحابية المستضافة، والاشتراك، ومزامنة الذاكرة.
- تحميل العملاء للنماذج المحلية وإلغاء تحميلها (يمكنهم الآن عرض النماذج، واختيار نموذج من مزوديك، واقتراح مزود جديد لتوافق عليه).
- تثبيت إضافة من رابط (المجلدات فقط)، والمهارات على مستوى المشروع.
- لم تُجرَّب أي قناة مراسلة على حساب حقيقي؛ تُختبر على خدمات تجريبية.

الاختبارات: `cargo test --workspace` (يُتخطى اختبار نموذج التضمين الحقيقي بالخيار `-- --skip real_embeddinggemma`). الترخيص: PolyForm Noncommercial 1.0.0 (راجع `LICENSE` و`COMMERCIAL-LICENSING.md`)؛ ويحتفظ `third_party/` بتراخيصه الخاصة.
