import asyncio
import json
from array import array

class Universe():
	def __init__(self, number_rows, number_columns, players):
		self.number_rows = number_rows
		self.number_columns = number_columns
		self.players = players

		self.grid = array('b', [0]*100)

	def matrix_to_plane(self, x, y):
		return [y*self.number_columns + x, self.grid[y*self.number_columns + x]]
	
	def plane_to_matrix(self, n):
		return [n % self.number_rows, n // self.number_columns, self.grid[n]]
	
	def update_grid(self, x, y, data):
		self.grid[y*self.number_columns + x] = data

	def get_neigbhor(self, cell, dir): # dir sigue el sistema horario

		# -------------
		# - 7 - 0 - 1 -
		# - 6 - X - 2 -
		# - 5 - 4 - 3 -
		# -------------

		# La ventaja de este sistema es que permite el `for`
		# También ayuda a simplificar cálculos (un poquito)

		match dir:
			case 7 | 0 | 1:
				cell -= self.number_columns
			case 1 | 2 | 3:
				cell += 1
			case 5 | 4 | 3:
				cell += self.number_columns
			case 5 | 6 | 7:
				cell -= 1
		
		return cell

universo = Universe(10, 10, ["Jaime"])
universo.matrix_to_plane(0, 1)
universo.update_grid(0, 1, 2)
print(universo.plane_to_matrix(universo.get_neigbhor(0, 4)))

exit()

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