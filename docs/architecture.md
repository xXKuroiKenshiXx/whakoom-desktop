# Arquitectura

## Interfaz y procesos

`app.rs` implementa la navegación y las pantallas con egui/eframe y OpenGL. Las tareas de red y decodificación se ejecutan fuera del hilo de interfaz. La ventana de verificación WebView2 se usa únicamente en Windows cuando el sitio exige pasos adicionales; la interfaz principal es nativa.

| Módulo | Responsabilidad |
| --- | --- |
| `api.rs`, `catalog.rs` | HTTPS, servicios web, lectura de fichas y paginación |
| `sync.rs` | Intenciones pendientes, reintentos y reconciliación |
| `storage.rs`, `session.rs` | Biblioteca, respaldos, escritura atómica y credenciales |
| `account.rs`, `social.rs` | Formularios de cuenta, perfiles y actividad |
| `discussion.rs`, `reactions.rs` | Opiniones y prioridades personales |
| `covers.rs` | Descarga, resolución y límites de caché |
| `calendar.rs`, `notifications.rs` | Fechas, selector y bandeja de avisos |
| `theme.rs`, `icons.rs`, `brand.rs`, `rating.rs`, `holographic.rs` | Componentes visuales |

## Conector

Las solicitudes autenticadas se restringen a HTTPS en `www.whakoom.com`, incluidas las redirecciones. Se usan contratos del cliente web como `/mycollection/comics.aspx/List` y `/wkws.asmx/…`, además de HTML público y formularios con token antifalsificación. No es una API oficial estable.

Las imágenes usan otro cliente, sin cookies, restringido al sitio y sus CDN `iN.whakoom.com`. Hay límites de respuesta, dimensiones y asignación de memoria al decodificar. La caché controla espacio, cantidad, calidad y miniaturas en memoria; las vistas extensas dibujan filas visibles.

## Sincronización

Cada acción cambia primero la biblioteca y genera una intención durable de la cuenta activa. Una confirmación tardía sólo puede reconocer esa misma intención: no elimina una edición posterior. Los errores conservan el pendiente y usan espera progresiva. Importar consulta toda la paginación antes de reconciliar; una respuesta incompleta no sustituye la biblioteca.

Las series prueban la operación conjunta y vuelven a leer el servidor. Los faltantes se reanudan en tandas de hasta 12, con hasta tres escrituras simultáneas y comprobación final. Los cambios individuales tienen prioridad frente a un reintento antiguo de la serie.

Colección y deseados se consultan completos. Lecturas y valoraciones se recuperan al consultar fichas, sin descargar todo el historial. Las notas se envían sólo cuando el servicio concede permisos. Etiquetas, importes, objetivos y reacciones son locales y están incluidos en el respaldo.

## Datos y sesión

Windows usa `%LOCALAPPDATA%\WhakoomDesktop`; reutiliza `%LOCALAPPDATA%\WhakoomNative` si existe una instalación previa. Linux usa `$XDG_DATA_HOME/whakoom-desktop` o `~/.local/share/whakoom-desktop`.

`WHAKOOM_DESKTOP_DATA_DIR` permite una carpeta alternativa. `QOMIC_DATA_DIR` se admite por compatibilidad. La carpeta contiene `settings.json`, `libraries/`, `covers/` y `pages/`. Las bibliotecas y pendientes se separan por cuenta.

La contraseña no se guarda. Windows utiliza `session.dpapi` protegido por DPAPI; Linux utiliza Secret Service. El perfil opcional de navegador Windows está en `webview/`. Borrar cookies no borra biblioteca ni caché. Los respaldos contienen datos personales, sin credenciales: conserválos fuera del repositorio.

## Verificación manual

`whakoom-check` ofrece consultas públicas y pruebas de lectura con la sesión existente: `--account`, `--verify-desktop`, `--inspect-pending`, `--covers`, `--detail-url URL`, `--friends-user USUARIO` y `--edition-query TEXTO`.

`--verify-keyring` requiere una carpeta de datos aislada y comprueba persistencia con una sesión ficticia. `--verify-session-stdin` permite comprobar una sesión transferida por stdin, sin guardarla. `--make-preview` genera una biblioteca de ejemplo con títulos públicos.

**`--verify-sync` envía los mismos valores existentes de un tomo a la cuenta real y los relee.** No lo uses sin intención de ejecutar esa comprobación. `--inspect RUTA --output ARCHIVO` puede guardar HTML autenticado: ese archivo es privado y no se publica.

Las capturas se generan con `--headless-preview --offline --preview-tab library --smoke RUTA.png`, en una carpeta de datos aislada. Las pruebas automatizadas no requieren una cuenta real.

Consultá [seguridad](../SECURITY.md), [validación](../VALIDATION.md) y [contribuciones](../CONTRIBUTING.md).
