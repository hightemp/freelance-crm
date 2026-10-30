<p align="center">
  <img src="docs/brand/icon.svg" alt="" width="128" height="128">
</p>

<h1 align="center">freelance-crm</h1>

<p align="center">
  Локальная настольная CRM для фрилансеров: клиенты, проекты, задачи, календарь,
  учёт времени, счета и зашифрованное хранилище в одном приложении, которое
  держит ваши данные на вашем компьютере.
</p>

<p align="center"><a href="README.md">English</a></p>

<p align="center">
  <a href="https://github.com/hightemp/freelance-crm/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/hightemp/freelance-crm/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
  <img alt="Platforms: Linux, Windows, macOS" src="https://img.shields.io/badge/platforms-Linux%20%7C%20Windows%20%7C%20macOS-lightgrey.svg">
  <img alt="Tauri 2" src="https://img.shields.io/badge/Tauri-2-24C8DB.svg?logo=tauri&logoColor=white">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-stable-orange.svg?logo=rust">
  <img alt="Svelte 5" src="https://img.shields.io/badge/Svelte-5-FF3E00.svg?logo=svelte&logoColor=white">
</p>

## Возможности

- **Клиенты и контакты** — компании и люди, контактные лица, способы связи,
  свои поля, теги, заметки, реквизиты с проверкой по правилам страны
  (Россия, США, Великобритания и международные), история звонков, встреч и
  сообщений с последующими задачами и событиями.
- **Проекты и задачи** — почасовая, фиксированная оплата или ретейнер; канбан-доска
  с настраиваемыми колонками, чек-листы, дедлайны и быстрое добавление (Ctrl/Cmd+N).
- **Календарь** — месяц, неделя, день и повестка; повторяющиеся события с
  часовыми поясами, напоминания системными уведомлениями, дедлайны задач,
  проектов и счетов отдельными слоями; импорт и экспорт ICS.
- **Учёт времени** — таймер в шапке и в трее, ручные записи, отчёты по дням,
  проектам или задачам, округление и экспорт в CSV.
- **Финансы** — счета из учтённого времени с автоматической нумерацией,
  PDF-документы (российский и международный шаблоны, логотип, подпись и печать),
  частичные оплаты, расходы с чеками, сводка доходов, расходов и
  неоплаченных счетов по валютам, а также оценка налогов по желанию (НПД, УСН или
  свои правила).
- **Хранилище** — пароли, SSH-ключи и другие секреты, зашифрованные
  мастер-паролем (Argon2id, XChaCha20-Poly1305), ключ восстановления,
  автоблокировка, генератор паролей; файлы тоже можно хранить зашифрованными.
- **Поиск** — глобальный полнотекстовый поиск и палитра команд (Ctrl/Cmd+K);
  на главном экране — просроченные задачи и задачи на сегодня, ближайшие
  события, неоплаченные счета и недавние клиенты.
- **Бэкапы** — один зашифрованный файл `.fcrm-backup` со всеми данными и файлами,
  бэкапы по расписанию с ротацией, восстановление с сохранением копии
  заменяемых данных, экспорт несекретных данных в JSON и CSV.
- Интерфейс на английском и русском, светлая и тёмная темы, без телеметрии.

## Документы

| | Английский | Русский |
|---|---|---|
| Требования к продукту | [PRD.md](PRD.md) | [PRD.ru.md](PRD.ru.md) |
| План задач | [TASKS.md](TASKS.md) | [TASKS.ru.md](TASKS.ru.md) |

- [Руководство пользователя](docs/user-guide.ru.md)
- [Безопасность](docs/security.md): что шифруется и какими ключами (на английском)
- [Как добавить язык](docs/adding-a-language.md) (на английском)
- Архитектурные решения: [docs/adr](docs/adr)

## Разработка

Нужны: Rust (stable), Node 22, pnpm 10 и
[системные зависимости Tauri](https://tauri.app/start/prerequisites/).
Для `cargo deny` нужен [cargo-deny](https://github.com/EmbarkStudios/cargo-deny)
(`cargo install --locked cargo-deny`).

```sh
make install       # dependencies and git hooks
make dev           # run the app with hot reload
make test          # Rust and frontend tests
make verify        # every check CI runs: fmt, clippy, tests, coverage, cargo-deny, lint, types
make e2e           # end-to-end tests against the real app
make build         # release installers for this platform
make help          # all targets
```

Те же команды доступны через pnpm (`pnpm tauri dev`, `pnpm verify`,
`pnpm verify --quick`, `pnpm e2e`) и cargo.

Сквозные тесты используют WebdriverIO со встроенным сервером WebDriver, поэтому
внешний драйвер не нужен ни в Linux, ни в Windows, ни в macOS. Они работают во
временном каталоге данных (`FREELANCE_CRM_DATA_DIR`) и никогда не трогают вашу
настоящую базу.

`make install` устанавливает git-хуки (lefthook): при коммите проверяются
форматирование и линтер для проиндексированных файлов, перед push запускается
`pnpm verify --quick`.

Проект разрабатывается через тесты: каждое изменение начинается с падающего теста.

Иконка приложения нарисована в [docs/brand/icon.svg](docs/brand/icon.svg);
`make icons` собирает из неё иконки для всех платформ (нужен
[resvg](https://github.com/linebender/resvg)).

## Структура

- `crates/crm-core` — бизнес-логика, база данных, криптография, бэкапы; без зависимости от Tauri
- `crates/crm-pdf` — документы счетов, которые рендерятся через Typst
- `src-tauri` — тонкий слой Tauri: команды, состояние приложения, трей, планировщик, плагины
- `src` — фронтенд SvelteKit SPA (Svelte 5, Tailwind CSS 4)
- `e2e` — сквозные тесты
- `messages` — переводы интерфейса

## Данные и безопасность

Всё хранится локально: в базе SQLite и в папке с файлами в каталоге данных
приложения. Секреты зашифрованы ключом, который открывается только вашим
мастер-паролем (или ключом восстановления); в бэкапах они тоже остаются
зашифрованными. Ничего никуда не отправляется.

## Лицензия

[MIT](LICENSE)
