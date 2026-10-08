# Validación de 2.0.0

Revisión: 8 de octubre de 2026. Los resultados describen las plataformas y rutas comprobadas; no constituyen una garantía de ausencia de errores.

## Comprobaciones automatizadas

100 pruebas aprobadas en Windows x64/MSVC y 100 en Ubuntu 24.04 x86_64. Formato y Clippy aprobados. RustSec: 0 vulnerabilidades clasificadas como tales; los avisos restantes y su alcance están documentados en [SECURITY.md](SECURITY.md).

- Visor: clic sobre imagen separado de la navegación a ficha/serie; Escape cierra el visor conservando la sección. Capturas nativas de zoom en Windows y AppImage Linux.
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
- Miniaturas progresivas: un tamaño pequeño permanece visible mientras falta la resolución final, incluso si su caché está corrupta; no se agranda antes de descargar la imagen final.
- Caché y respaldos: límites, limpieza de archivos propios, imágenes inválidas, exportación CSV y validación de importaciones.
- Linux: escritura atómica no sigue un enlace simbólico al reemplazarlo; el archivo resultante tiene permisos 0600.

## Ejecución y paquetes

- Windows: interfaz nativa y ejecutable 2.0 comprobados con capturas de catálogo, estadísticas, ficha, visor, asistente de calidad y ayuda pública. El instalador se compiló con NSIS y su ejecutable extraído coincide por SHA-256 con el binario final; la instalación/desinstalación en una carpeta temporal fue comprobada en 1.0.
- Linux: ejecución en Ubuntu 24.04 dentro de WSL con Xvfb y renderizado software X11. La AppImage 2.0.0 abrió el catálogo reorganizado y el visor de portada; asistente de calidad, biblioteca y Ajustes en chino se comprobaron en las revisiones anteriores. Wayland y otras distribuciones no se probaron de forma interactiva.
- Secret Service: una sesión ficticia se guardó, recuperó y borró en un llavero aislado. La fecha de lectura utiliza la zona horaria del sistema.
- Conector Linux: sesión real transferida únicamente por stdin, sin archivo ni argumentos con credenciales; lectura de colección, amigos y siete secciones de cuenta.
- Capturas Windows: biblioteca y ficha con sinopsis de color, controles de colección y separación de portada. Notas largas quedan dentro de un área con scroll.
- Dependencias de empaquetado AppImage verificadas por SHA-256. Los binarios no tienen firma comercial.

## Límites de la verificación

Los formularios de ayuda Windows requieren WebView2 y una sesión oficial de Zendesk; en Linux abren el navegador. El envío final de reportes/comentarios no se ejercitó con mensajes de prueba. Las opiniones tienen pruebas de contrato y lectura real del formulario; su publicación no se ejercitó para evitar modificar la cuenta.

Los formularios de cuenta tienen pruebas aisladas y consultas reales de lectura. No se cambian contraseñas, pagos, suscripciones ni privacidad para probarlos. No se eliminan colecciones reales como prueba de las bajas. La verificación social consulta personas seguidas y actividad pública; no presupone un historial completo.

El inicio de sesión con credenciales y la interfaz comparten código entre plataformas. El navegador de verificación adicional es específico de Windows. Linux necesita Secret Service para persistir la sesión y un entorno gráfico con OpenGL; no se afirma compatibilidad con todas las distribuciones o controladores.

Las capturas del README usan una carpeta aislada, títulos públicos y datos de biblioteca de ejemplo. Credenciales, bibliotecas reales y diagnósticos personales quedan fuera del repositorio y de los paquetes.
