# Seguridad y privacidad

La contraseña de Whakoom no se persiste. Windows DPAPI protege la sesión, biblioteca, notas, perfiles, cola de sincronización, preferencias y páginas privadas con el usuario local de Windows. Linux guarda la sesión y una clave de archivos privados en Secret Service; los archivos usan AES-256-GCM con un nonce aleatorio por escritura. Secret Service debe estar desbloqueado: ante un fallo se informa el error y no se guarda una copia en texto plano.

Los archivos JSON anteriores de `libraries`, `pages` y `settings.json` se migran en segundo plano mediante reemplazo atómico. Un error detiene la migración y se muestra en la app; los archivos que aún no se migraron permanecen como estaban. No se recorren enlaces simbólicos ni archivos de más de 64 MB. Los archivos cifrados se vinculan a su sección/nombre lógico; un archivo dañado o una clave distinta no se sobrescribe con valores por defecto. El cifrado no borra copias anteriores que hayan quedado en respaldos del sistema, papelera, sincronizadores o sectores libres del disco.

Estos mecanismos protegen datos en reposo, no frente a malware, otro programa con control del mismo usuario, capturas de pantalla ni datos presentes en memoria mientras la app está abierta. Las portadas y avatares públicos mantienen una caché de imágenes normal. Las escrituras usan archivos temporales únicos y reemplazo atómico; en Linux los archivos creados tienen permisos 0600.

Los respaldos `.whakoom` usan AES-256-GCM autenticado y PBKDF2-HMAC-SHA256 con 600 000 iteraciones, una sal de 16 bytes y nonce de 12 bytes generados con el sistema operativo. Su contraseña es independiente de Whakoom y no se guarda. El formato permite restaurar en otro sistema operativo o en la web. No existe recuperación de contraseña. JSON y CSV son exportaciones **sin cifrar**, identificadas como tales en la interfaz; no contienen sesión pero sí biblioteca, notas, perfiles y cambios pendientes.

Las cookies sólo acompañan solicitudes HTTPS al dominio exacto `www.whakoom.com`. Las portadas y avatares usan un cliente separado sin cookies. Se comprueban redirecciones, límites de respuesta, tamaño de imágenes y enlaces importados. Las fotos personales se normalizan y se almacenan dentro del respaldo; sus referencias no son rutas arbitrarias. Crear listas y cambiar sus favoritos requiere verificar la identidad de la sesión antes de escribir.

La cola está separada por cuenta. Sólo se elimina una intención cuando se confirma ese mismo cambio; una respuesta tardía no debe descartar una edición posterior. Un error de permisos se conserva para que el usuario pueda revisarlo.

Las solicitudes del conector y sus imágenes comparten un límite de dos conexiones simultáneas, con al menos 300 ms entre inicios; Listado Manga tiene un límite independiente equivalente. HTTP 429 pausa las solicitudes del sitio según `Retry-After` (segundos o fecha HTTP); si falta, aplica espera creciente desde 30 segundos. No se intenta eludir el límite cambiando IP, identidad o cookies. La caché continúa disponible y los cambios pendientes no se descartan durante la pausa.

Reportá vulnerabilidades por el [canal privado del repositorio](https://github.com/xXKuroiKenshiXx/whakoom-desktop/security/advisories/new), con una reproducción mínima sin credenciales ni datos reales.

## Dependencias revisadas

RustSec se consulta con `cargo audit`. En la revisión del 9 de octubre de 2026 no se reportaron vulnerabilidades clasificadas como tales. El lockfile conserva avisos que se explican sin ocultarlos:

- `fxhash 0.2.1` (RUSTSEC-2025-0057): aviso de mantenimiento, dependencia de `scraper`/`selectors`, sí utilizada.
- `glib 0.18.5` (RUSTSEC-2024-0429): aviso de unsoundness; `proc-macro-error 1.0.4` (RUSTSEC-2024-0370): mantenimiento. Llegan al lockfile por las dependencias Linux de Wry. Wry está declarado sólo para Windows y utiliza WebView2 allí: GLib y esas macros **no están en el árbol compilado de Windows ni en el de Linux**. No se habilita Wry en Linux.

Los avisos deben revisarse de nuevo al actualizar dependencias o plataformas. La auditoría automática y las pruebas no prueban ausencia de vulnerabilidades. Los binarios publicados actualmente no tienen firma de código comercial.

Cuenta consulta un token antifalsificación fresco del formulario correcto antes de enviar cambios. Los campos de contraseña se borran del formulario al enviarlos y se limpian de las estructuras temporales; no se guardan en la biblioteca, sesión ni respaldos. Los tokens tampoco forman parte de los datos exportables. Borrar cookies desconecta la sesión y elimina sólo el perfil de navegador asociado a la app, comprobando su ubicación antes de borrarlo.

La comunidad Zendesk usa un cliente sin credenciales y un dominio exacto distinto. Las respuestas tienen límites y su HTML se representa como texto. Los formularios oficiales Windows se abren en un contexto incógnito independiente, sin puente de comandos nativos; en Linux se usan enlaces validados al navegador. Borrar cookies descarta también el contexto de ayuda. Las opiniones públicas sólo se encolan al pulsar Publicar y se confirman releyendo la opinión personal.

## Actualizaciones y sitios externos

El actualizador consulta las releases estables de `xXKuroiKenshiXx/whakoom-desktop` por HTTPS. Exige el nombre exacto del paquete, URL del repositorio, tamaño limitado y SHA-256 de la publicación; verifica el archivo de nuevo antes de instalar. No ejecuta instrucciones de las notas de una release ni instala al comprobar novedades. Las conexiones de actualización y Listado Manga no usan cookies de Whakoom. El hash verifica integridad respecto de GitHub, no sustituye una firma de código.

Listado Manga se muestra como texto y enlaces permitidos en la vista nativa. La página original Windows usa un WebView2 separado, sin IPC ni sesión del conector de Whakoom. El selector de compra sólo abre enlaces HTTPS permitidos y búsquedas codificadas; no envía credenciales ni realiza pagos.


## Versión web

La web necesita desplegar su frontend y sus funciones de servidor. El formulario de login envía las credenciales por HTTPS a este servidor, que consulta el formulario de Whakoom con un token antifalsificación fresco. No registra ni conserva la contraseña. La sesión se cifra con AES-256-GCM en una cookie HttpOnly, Secure y SameSite=Strict que expira en dos horas. La clave de servidor se configura fuera del repositorio; rotarla revoca las sesiones emitidas. El administrador del alojamiento puede acceder al proceso del servidor: es necesario confiar en ese alojamiento.

Las funciones restringen los destinos a HTTPS en www.whakoom.com, también durante las redirecciones. Las escrituras validan el origen y el tipo y tamaño del cuerpo; los identificadores de cambios se obtienen de una ficha consultada en el servidor. El login está limitado a cinco intentos por minuto e IP y el resto de solicitudes tiene su propio límite. HTTP 429 se respeta con una pausa, sin reintentos automáticos en bucle. No se eluden verificaciones ni permisos del servicio.

IndexedDB contiene sólo el respaldo cifrado, con una clave independiente no exportable que permanece en memoria. El service worker guarda la interfaz pública y excluye las respuestas de cuenta. No hay CORS abierto, scripts de terceros ni analítica. El cifrado del navegador no protege de un servidor que entregue JavaScript comprometido ni de extensiones maliciosas. Los datos importados se representan como texto y las imágenes se restringen a HTTPS de Whakoom.

Las operaciones online se confirman leyendo el estado de Whakoom. Las notas, fechas, importes, reacciones, personas favoritas y logros son locales y se incluyen en el respaldo cifrado. La web no almacena una biblioteca privada en una base de datos compartida ni ofrece un ranking público. Las pruebas simulan el servicio: el despliegue definitivo debe verificar que Whakoom permite iniciar sesión desde ese alojamiento.
