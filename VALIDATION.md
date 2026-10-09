# Validación de 3.1.0

Revisión: 8 de octubre de 2026. Los resultados describen las plataformas y rutas comprobadas; no constituyen una garantía de ausencia de errores.

## Comprobaciones de 3.1.0

Revisión adicional de solicitudes: 139 pruebas aprobadas en Windows x64/MSVC, formato y Clippy sin advertencias. Las pruebas comprueban la pausa compartida ante 429, las dos formas de `Retry-After`, la conservación del plazo frente a respuestas simultáneas y la carga de sólo dos previews visibles sin repetirlas. Las portadas resueltas sobreviven a la llegada de nuevas variantes de búsqueda. La geometría de las cuatro acciones de serie se comprueba por igualdad de ancho y alto. Las comprobaciones Linux siguientes corresponden a la revisión anterior.

131 pruebas aprobadas en Windows x64/MSVC y 131 en Ubuntu 24.04 x86_64. Formato y Clippy aprobados. Capturas nativas del perfil, popup y faltantes, incluidas ventanas pequeñas; AppImage ejecutada en X11 con Xvfb y renderizado software. El ejecutable extraído del instalador coincide por SHA-256 con el binario final.

- Revisión de interfaz: Deseados filtra Todos, Series y Tomos sin excluir tipos del estado «Lo quiero». Lecturas alterna por clic entre lista y portadas. Los favoritos de Personas persisten en el respaldo sin modificar Seguidos, Seguidores ni la cola de cambios online.
- Valoraciones personales compactas: clic para votar y repetir clic para quitar la nota. Comprar conserva el tamaño y la alineación de Leído y abrir las tiendas no altera la colección.
- Listado Manga combina hasta seis variantes y limita a dos las solicitudes de red simultáneas, deduplica y prioriza coincidencias. Consulta pública real anterior: seis variantes del título A Returner’s Magic Should Be Special devolvieron 75 resultados únicos; se comprobaron portadas de las primeras seis fichas. Las imágenes usan un cliente sin cookies de Whakoom.
- Amazon: consulta autenticada de sólo lectura de las tiendas de un tomo; el enlace `clickgotoshop.ashx` devolvió JSON y se resolvió a una URL HTTPS de Amazon con precio. La respuesta rechaza hosts externos ajenos a Amazon; no se efectuaron compras.
- Progreso de propiedad: el total procede de una edición completa; no se estima usando sólo los tomos adquiridos. Respeta el estado local pendiente y no muestra cifras inventadas cuando falta el total.
- Consulta autenticada de sólo lectura de la edición 627715: el servicio `EditionComicsPage` con modo 1 y las páginas `/todos`, `/tengo` y `/faltan` coinciden (2 tomos, 1 adquirido y 1 faltante en la cuenta consultada). La ficha del tomo identifica la misma edición. No se efectuaron altas, bajas ni cambios de perfil durante esta comprobación.
- Tomos faltantes considera sólo colecciones completas, separa ediciones conocidas y conserva cambios locales pendientes de confirmación. Sugiere el siguiente tomo publicado o un hueco anterior; no inventa números de tomos.
- Filtros de edición: paginación automática, deduplicación y conservación de la última caché completa durante cargas parciales. Respuestas atrasadas o de otra cuenta no se incorporan a la biblioteca.
- Cuenta: navegación sobre el resumen; edición del perfil mediante modal, cierre con Escape y conservación del borrador cuando termina la subida de una foto. La edición continúa usando los formularios autenticados y su verificación CSRF.
- Pruebas de interfaz comprueban navegación real por clics, geometría de las tarjetas y filtros sin alterar notas o propiedad. Los controles nuevos tienen traducciones en los cinco idiomas.
- RustSec: 0 vulnerabilidades registradas en la revisión; permanecen los tres avisos informativos documentados. La consulta de paquetes retirados del registro sufrió tiempos de espera y no pudo verificarse por completo.

## Comprobaciones de 3.0.0

- Deseados: unión de `/buscados`, servicio paginado y sección Buscados del perfil propio. Las fichas conocidas que faltan se vuelven a consultar; no se recuperan como deseados si ya fueron quitadas online. La lectura real devolvió 13 elementos actuales. Tomos y series mantienen su tipo y sus IDs originales.
- Favoritos de series y «Lo quiero» comparten el estado online: la intención más reciente sustituye a la anterior, una confirmación tardía no la borra y la reconciliación conserva los cambios pendientes.
- Catálogo: botón independiente de Usuarios con solicitudes específicas; los iconos de catálogo y listas se actualizaron.
- Ventanas bajas: entrada de rueda en egui comprueba que el encabezado completo de la serie y sus tomos se desplazan juntos. La cuadrícula mantiene virtualización y sus tarjetas siguen recibiendo clics.
- Almacenamiento: las dos tarjetas tienen igual ancho y alto; una prueba de 100 repintados verifica que no crezcan continuamente. Se apilan en ventanas estrechas.
- Importes: persistencia y exportación de monedas ISO; los totales agrupan las divisas y conservan los importes históricos sin moneda asignada.
- Listado Manga: búsquedas reales, 3477 colecciones navegables y ficha pública contrastadas con el sitio. Sus imágenes usan un cliente sin cookies. Se rechazan hosts, redirecciones y respuestas que excedan los límites.
- Tiendas: lectura real de la opción Amazon de Whakoom. Mercado Libre codifica título y número según el país de la moneda; se rechazan enlaces ajenos a las tiendas admitidas. No se realizó ninguna compra.
- Actualizaciones: metadatos exclusivos del repositorio, versión estable, digest obligatorio, descarga con SHA-256, rechazo de archivos alterados y resultados tardíos. Descarga real de la release previa verificada en una carpeta aislada.
- Windows portable: reemplazo y reinicio ejecutados con dos binarios de prueba aislados. El asistente esperó la salida, verificó el hash con .NET, conservó el binario anterior y arrancó el nuevo. No se reemplazó el ejecutable del usuario durante esta prueba.
- Linux: una copia aislada de la AppImage 2.0.5 se reemplazó por 3.0.0, se verificaron los bytes y se comprobó que abrió su ventana en X11. El paquete incluye las bibliotecas de teclado cargadas dinámicamente que no detectaba el empaquetador.
## Comprobaciones previas conservadas

- Deseados: consulta autenticada de `/buscados` y del servicio completo, unión sin duplicados y aceptación de su marcador de fin numérico. La consulta real no modifica la cuenta.
- Perfiles: Comicteca (primera y segunda página), Buscados, Listas y búsqueda de usuarios contrastados con el sitio. Prueba aislada de resultados tardíos y perfiles de otra persona: no se importan a la biblioteca propia.
- Fechas de compra: nuevas altas de series registran el día y conservan sin fecha los tomos que ya estaban adquiridos. No se encontró una fecha histórica fiable en las páginas consultadas.
- Interfaz: botones sociales mayores, actividad de ambas relaciones, carrusel ampliado, controles de año centrados y tarjetas de Ajustes con ancho uniforme.

## Comprobaciones automatizadas

115 pruebas aprobadas en Windows x64/MSVC y 115 en Ubuntu 24.04 x86_64. Formato y Clippy aprobados. RustSec: 0 vulnerabilidades clasificadas como tales; los avisos restantes y su alcance están documentados en [SECURITY.md](SECURITY.md).

- Visor: en búsqueda, tanto portadas como lista abren la ficha al pulsar una imagen. El visor se abre únicamente desde la portada dentro de la ficha; Escape lo cierra conservando el cómic abierto. Capturas nativas de zoom en Windows y AppImage Linux.
- Carrusel: un contacto y ventana de 760 px mantienen el movimiento y los límites de la sección; los controles de animación conservan su comportamiento.
- Opiniones: contratos actuales de tomos y series, Unicode y límite UTF-16, persistencia del pendiente y actualización de su valoración durante reintentos.
- Metadatos: ISBN múltiple, cantidad de propietarios y votos son datos distintos; ausencia de información no se convierte en una cifra inventada.
- Estadísticas: lectura de gráficos autorizados, rechazo del informe restringido, fechas reales de compra y sustitución mensual sin duplicar las lecturas locales.
- Zendesk: enlaces restringidos, lectura del campo real `details`, texto sin ejecución de HTML, categorías/publicaciones/respuestas contrastadas con la API pública.
- Consultas autenticadas de sólo lectura a fichas y formularios de opinión; estadísticas originales restringidas para la cuenta de prueba. No se publicaron opiniones ni reportes para comprobarlas.
- Formato, pruebas de todos los targets y Clippy con advertencias como errores en Windows y Linux.
- Navegación desde Notificaciones durante una actualización de colección, orden de la barra lateral y liberación del estado de sincronización ante un error tardío.
- Carruseles infinitos de Amigos y Seguidores: avance con pocos contactos, vuelta al inicio sin invertir el sentido, un único contacto y desactivación de animaciones.
- Asistente inicial: elección de caché, calidad alta por defecto, volver conservando la elección y aplicar únicamente al finalizar. Capturas nativas de ambos pasos con fondo atenuado.
- Calendario: clics que avanzan tres meses sin cerrar el selector ni modificar la fecha hasta elegir un día.
- Biblioteca: eliminación de una serie conserva notas y lecturas, actualiza la vista y genera cambios pendientes.
- Sincronización: errores parciales, confirmaciones tardías, cambios nuevos durante una tanda, aislamiento de cuentas y paginación incompleta.
- Conector: HTML actual, formularios, tokens antifalsificación, campos permitidos, URLs y redirecciones.
- Explorar y Listas: cuatro categorías con 56 fichas por página, listas propias/favoritas y lista pública de 54 tomos en dos páginas comprobadas contra el sitio actual. Creación y favoritos se verifican con contratos y validación de solicitudes; no se crean listas personales como prueba.
- Tipografía: presencia de los glifos matemáticos solicitados, chino y cirílico en las fuentes incluidas. Capturas del catálogo en inglés y de Ajustes en chino.
- Herramientas locales: relecturas, ubicación, conservación, orden de lectura y fotos sobreviven al respaldo; referencias de fotos y fechas corruptas se rechazan.
- Seguidores y perfiles comprobados en el sitio real; pestañas, respaldos y aislamiento de respuestas de otra cuenta verificados. No se publican los resultados personales de la consulta.
- Hover: margen exterior de 18 px que mantiene el zoom sin activarlo desde fuera; desvanecimiento al salir y desactivación inmediata de animaciones.
- Compresión sin pérdida: mismos píxeles tras guardar, archivo igual o menor, caché borrada no recreada por el optimizador. Ajustes organizado por secciones y capturas de Almacenamiento.
- Miniaturas progresivas: un tamaño pequeño ya guardado permanece visible mientras falta la resolución final, incluso si su caché está corrupta. Una portada sin caché descarga directamente la calidad elegida para evitar dos solicitudes.
- Caché y respaldos: límites, limpieza de archivos propios, imágenes inválidas, exportación CSV y validación de importaciones.
- Linux: escritura atómica no sigue un enlace simbólico al reemplazarlo; el archivo resultante tiene permisos 0600.

## Ejecución y paquetes

- Windows: interfaz nativa y ejecutable 2.0.5 comprobados con capturas de catálogo, estadísticas, ficha, visor, asistente de calidad y ayuda pública. El instalador se compiló con NSIS y su ejecutable extraído coincide por SHA-256 con el binario final; la instalación/desinstalación en una carpeta temporal fue comprobada en 1.0.
- Linux: ejecución en Ubuntu 24.04 dentro de WSL con Xvfb y renderizado software X11. La AppImage 2.0.5 abrió Seguidores y su actividad con el carrusel ampliado; catálogo y visor se comprobaron en 2.0.0; asistente de calidad, biblioteca y Ajustes en chino se comprobaron en las revisiones anteriores. Wayland y otras distribuciones no se probaron de forma interactiva.
- Secret Service: una sesión ficticia se guardó, recuperó y borró en un llavero aislado. La fecha de lectura utiliza la zona horaria del sistema.
- Conector Linux: sesión real transferida únicamente por stdin, sin archivo ni argumentos con credenciales; lectura de colección, amigos y siete secciones de cuenta.
- Capturas Windows: biblioteca y ficha con sinopsis de color, controles de colección y separación de portada. Notas largas quedan dentro de un área con scroll.
- Dependencias de empaquetado AppImage verificadas por SHA-256. Los binarios no tienen firma comercial.
- 3.0.0: capturas nativas Windows de biblioteca, catálogo, usuarios, ficha, tiendas, Listado Manga, estadísticas y Ajustes. AppImage final ejecutada en X11 con Xvfb y renderizado software; los ensayos previos de esta revisión también utilizaron el entorno gráfico de WSLg.
- Instalador 3.0.0 compilado y extraído: su ejecutable coincide por SHA-256 con la compilación final. El flujo de actualización del instalador existente no se ejecutó contra una instalación real; el reemplazo portable sí se probó de principio a fin.

## Límites de la verificación

Los formularios de ayuda Windows requieren WebView2 y una sesión oficial de Zendesk; en Linux abren el navegador. El envío final de reportes/comentarios no se ejercitó con mensajes de prueba. Las opiniones tienen pruebas de contrato y lectura real del formulario; su publicación no se ejercitó para evitar modificar la cuenta.

Los formularios de cuenta tienen pruebas aisladas y consultas reales de lectura. No se cambian contraseñas, pagos, suscripciones ni privacidad para probarlos. No se eliminan colecciones reales como prueba de las bajas. La verificación social consulta personas seguidas y actividad pública; no presupone un historial completo.

El inicio de sesión con credenciales y la interfaz comparten código entre plataformas. El navegador de verificación adicional es específico de Windows. Linux necesita Secret Service para persistir la sesión y un entorno gráfico con OpenGL; no se afirma compatibilidad con todas las distribuciones o controladores.

Las capturas del README usan una carpeta aislada, títulos públicos y datos de biblioteca de ejemplo. Credenciales, bibliotecas reales y diagnósticos personales quedan fuera del repositorio y de los paquetes.
