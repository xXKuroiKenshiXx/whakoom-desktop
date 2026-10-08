# Validación de 1.0

Revisión: 8 de octubre de 2026. Los resultados describen las plataformas y rutas comprobadas; no constituyen una garantía de ausencia de errores.

## Comprobaciones automatizadas

76 pruebas aprobadas en Windows x64/MSVC y 76 en Ubuntu 24.04 x86_64. Formato y Clippy aprobados. RustSec: 0 vulnerabilidades clasificadas como tales; los avisos restantes y su alcance están documentados en [SECURITY.md](SECURITY.md).

- Formato, pruebas de todos los targets y Clippy con advertencias como errores en Windows y Linux.
- Calendario: clics que avanzan tres meses sin cerrar el selector ni modificar la fecha hasta elegir un día.
- Biblioteca: eliminación de una serie conserva notas y lecturas, actualiza la vista y genera cambios pendientes.
- Sincronización: errores parciales, confirmaciones tardías, cambios nuevos durante una tanda, aislamiento de cuentas y paginación incompleta.
- Conector: HTML actual, formularios, tokens antifalsificación, campos permitidos, URLs y redirecciones.
- Caché y respaldos: límites, limpieza de archivos propios, imágenes inválidas, exportación CSV y validación de importaciones.
- Linux: escritura atómica no sigue un enlace simbólico al reemplazarlo; el archivo resultante tiene permisos 0600.

## Ejecución y paquetes

- Windows: interfaz nativa, instalador por usuario y ejecutable portable. Instalación en una carpeta temporal, identidad del binario instalado, apertura y desinstalación comprobadas.
- Linux: ejecución en Ubuntu 24.04 dentro de WSL con Xvfb y renderizado software X11. Se abrió la biblioteca y Ajustes desde la AppImage empaquetada. Wayland y otras distribuciones no se probaron de forma interactiva.
- Secret Service: una sesión ficticia se guardó, recuperó y borró en un llavero aislado. La fecha de lectura utiliza la zona horaria del sistema.
- Conector Linux: sesión real transferida únicamente por stdin, sin archivo ni argumentos con credenciales; lectura de colección, amigos y siete secciones de cuenta.
- Capturas Windows: biblioteca y ficha con sinopsis de color, controles de colección y separación de portada. Notas largas quedan dentro de un área con scroll.
- Dependencias de empaquetado AppImage verificadas por SHA-256. Los binarios no tienen firma comercial.

## Límites de la verificación

Los formularios de cuenta tienen pruebas aisladas y consultas reales de lectura. No se cambian contraseñas, pagos, suscripciones ni privacidad para probarlos. No se eliminan colecciones reales como prueba de las bajas. La verificación social consulta personas seguidas y actividad pública; no presupone un historial completo.

El inicio de sesión con credenciales y la interfaz comparten código entre plataformas. El navegador de verificación adicional es específico de Windows. Linux necesita Secret Service para persistir la sesión y un entorno gráfico con OpenGL; no se afirma compatibilidad con todas las distribuciones o controladores.

Las capturas del README usan una carpeta aislada, títulos públicos y datos de biblioteca de ejemplo. Credenciales, bibliotecas reales y diagnósticos personales quedan fuera del repositorio y de los paquetes.
