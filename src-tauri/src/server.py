import asyncio
import json

class Player():
	def __init__(self, reader, writer, server):
		self.reader = reader
		self.writer = writer
		self.server = server

		self.addr = writer.get_extra_info("peername")
		self.name = "Unknow"
		self.status = 0

		self.last_command_recived = None
		self.last_command_sended = None

	async def send_json(self, data):
		msg = (json.dumps(data) + "\n").encode("UTF-8")
		self.writer.write(msg)
		
		await self.writer.drain()

		print(">Server> " + str(msg.decode("UTF-8")))

	async def close(self):
		print("Conexión cerrada con: " + self.name)
		self.writer.close()
		await self.writer.wait_closed()

	async def listen(self):
		print("Escuchando por: " + str(self.addr))
		try:
			while True:
				data = await self.reader.readline()

				if not data:
					break

				msg = json.loads(data.decode())
				command = msg.get("command")

				print("<Cliente< " + str(msg))
				if command == "init":

					temp_name = msg.get("data", "Uknown")

					colision = False
					for i in self.server.players:
						if i == temp_name:
							colision = True
							break

					if colision:
						await self.send_json({
							"command": "init",
							"data": ["Nombre repetido"],
							"status_code": False
						})

					else:
						self.name = temp_name
						self.status = 1

						await self.send_json({
							"command": "init",
							"data": [self.name], # antiguamente [self.name], modificado para pruebas
							"status_code": True
						})

				if command == "other_players":
					others = [
						p.name for p in self.server.players 
						if p.name != self.name and p.status == 1
					]
					
					await self.send_json({
						"command": "send_players",
						"data": others if len(others) != 0 else [],
						"status_code": True if len(others) != 0 else False
					})

		except Exception as e:
			print("Error con el cliente " + self.name + ": " + str(e))

		finally:
			self.close()

class Server():
	def __init__(self, host, port):
		self.host = host
		self.port = port

		self.players = []
		self.turn = 0

	async def callback_conexion(self, reader, writer):
		player = Player(reader, writer, self)
		self.players.append(player)

		await player.listen()

		self.players.remove(player)
	
	async def run(self):
		server = await asyncio.start_server(self.callback_conexion, self.host, self.port)
		print("Servidor iniciado en: " + self.host + ":" + str(self.port))

		async with server:
			await server.serve_forever()

ip = "127.0.0.1"
port = 5005

if __name__ == "__main__":
	server = Server(ip, port)
	asyncio.run(server.run())