import socket
import time

def start_test_client():
    ip = "127.0.0.1"
    port = 5005
    buffer_size = 1024
    name = "TestPlayer"

    try:
        # 1. Crear el socket y conectar
        client = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        client.connect((ip, port))
        print(f"[*] Conectado al servidor en {ip}:{port}")

        # 2. Enviar mensaje de "Nuevo cliente" (Siguiendo tu formato [Nombre]: Mensaje)
        # Es vital que incluyas el ": " para que tu servidor no rompa el split
        reg_msg = f"[{name}]: init"
        client.send(reg_msg.encode("UTF-8"))
        print(f"[>] Enviado: {reg_msg}")

        # Esperar un poco para simular actividad
        time.sleep(1)

        # 3. Pedir la lista de jugadores
        list_msg = f"[{name}]: other_players"
        client.send(list_msg.encode("UTF-8"))
        print(f"[>] Enviado: {list_msg}")

        # 4. Recibir respuesta
        response = client.recv(buffer_size)
        print(f"[<] Recibido del servidor: {response.decode('UTF-8')}")

        # Mantener la conexión abierta para que aparezca en la lista de los demás
        print("[*] Conexión mantenida. Presiona Ctrl+C para salir.")
        while True:
            time.sleep(1)

    except ConnectionRefusedError:
        print("[!] Error: El servidor no está encendido.")
    except Exception as e:
        print(f"[!] Error inesperado: {e}")
    finally:
        client.close()

if __name__ == "__main__":
    start_test_client()