# Juego-de-la-vida-con-Rust-Tauri-y-Python
En este proyecto, un profesor me pidió que crease un juego de la vida pero para 2 jugadores. Por lo que se requiere de una infraestructura de cliente-servidor-cliente, y, por curiosidad mía, varios lenguajes de programación funcionando simultáneamente.

Concretamente, estoy usando varias tecnologías. Estas son:

1. Python 3.14.2.
2. Rust.
3. Tauri 2.0.

Estas tecnologías me permiten tener el siguiente esquema de funcionamiento:

1. Desarrollo una página web con HTML, CSS y JS.
2. Un _script_ de JS invoca funciones de Rust.
3. Rust gestiona las llamadas y hace peticiones al servidor de Python.
4. El servidor procesa las llamadas y se comunica con un cliente. El proceso se invierte.

A menudo, una función de Rust es llamada por el _script_ de la aplicación web. Por ejemplo, si se desea presentar los usuarios que están en línea, será necesario implementar código asíncrono:

1. Se dispara la una función en el _frontend_.
2. Este invoca una función de Rust (que es asíncrona).
3. Rust le pide el contenido al servidor.
4. El servidor lo recopila y responde.
5. El _backend_ recibe la respuesta.
6. JavaScript recibe la respuesta, y la presenta.

La asincronía es crítica porque no podemos estar seguros de que el servidor responda inmediatamente. Si no estuviese implementada, entonces el programa se colgaría. Aquí también es donde entra el uso de los punteros inteligentes `ARC` y `MUTEX` en el `AppState` para manejar el cliente, que debe ser accedido por varias funciones (casi) al mismo tiempo.
