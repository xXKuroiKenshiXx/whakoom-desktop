# Seguridad y privacidad

La contraseña no se persiste. Windows DPAPI protege la sesión con el usuario local de Windows; Linux la guarda en Secret Service. Estos mecanismos no protegen frente a otros programas que ya controlen ese mismo usuario. Los respaldos JSON excluyen la sesión, pero contienen biblioteca, notas, perfiles y cambios pendientes. Las escrituras usan archivos temporales únicos y reemplazo atómico; en Linux los archivos creados tienen permisos 0600.

Las cookies sólo acompañan solicitudes HTTPS al dominio exacto `www.whakoom.com`. Las portadas y avatares usan un cliente separado sin cookies. Se comprueban redirecciones, límites de respuesta, tamaño de imágenes y enlaces importados. Las fotos personales se normalizan y se almacenan dentro del respaldo; sus referencias no son rutas arbitrarias. Crear listas y cambiar sus favoritos requiere verificar la identidad de la sesión antes de escribir.

La cola está separada por cuenta. Sólo se elimina una intención cuando se confirma ese mismo cambio; una respuesta tardía no debe descartar una edición posterior. Un error de permisos se conserva para que el usuario pueda revisarlo.

Reportá vulnerabilidades por el [canal privado del repositorio](https://github.com/xXKuroiKenshiXx/whakoom-desktop/security/advisories/new), con una reproducción mínima sin credenciales ni datos reales.

## Dependencias revisadas

RustSec se consulta con `cargo audit`. En la revisión de 1.0 no se reportaron vulnerabilidades clasificadas como tales. El lockfile conserva avisos que se explican sin ocultarlos:

- `fxhash 0.2.1` (RUSTSEC-2025-0057): aviso de mantenimiento, dependencia de `scraper`/`selectors`, sí utilizada.
- `glib 0.18.5` (RUSTSEC-2024-0429): aviso de unsoundness; `proc-macro-error 1.0.4` (RUSTSEC-2024-0370): mantenimiento. Llegan al lockfile por las dependencias Linux de Wry. Wry está declarado sólo para Windows y utiliza WebView2 allí: GLib y esas macros **no están en el árbol compilado de Windows ni en el de Linux**. No se habilita Wry en Linux.

Los avisos deben revisarse de nuevo al actualizar dependencias o plataformas. La auditoría automática y las pruebas no prueban ausencia de vulnerabilidades. Los binarios publicados actualmente no tienen firma de código comercial.

Cuenta consulta un token antifalsificación fresco del formulario correcto antes de enviar cambios. Los campos de contraseña se borran del formulario al enviarlos y se limpian de las estructuras temporales; no se guardan en la biblioteca, sesión ni respaldos. Los tokens tampoco forman parte de los datos exportables. Borrar cookies desconecta la sesión y elimina sólo el perfil de navegador asociado a la app, comprobando su ubicación antes de borrarlo.
