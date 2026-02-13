use std::net::Ipv4Addr;
use std::time::Duration;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader, AsyncWriteExt};
use tokio::net::{TcpStream, tcp::{OwnedReadHalf, OwnedWriteHalf}};
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};
use tauri::State;
use std::error::Error;

#[derive(Serialize, Deserialize, Debug)]
struct Request {
	command: String,
	data: Option<String>,
	status_code: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct Response {
	command: String,
	data: Option<Vec<String>>,
	status_code: bool,
}

struct Client {
	name: String,
	reader: Mutex<BufReader<OwnedReadHalf>>, // protege lectura
	writer: Mutex<OwnedWriteHalf>,           // protege escritura
}

impl Client {
	pub async fn new(name: String, ip: String, port: u16) -> Result<Self, Box<dyn Error + Send + Sync>> {
		let ip_addr: Ipv4Addr = ip.parse().expect("IP inválida");
		println!("Conectando con {}:{}", ip_addr, port);

		let mut cont: u8 = 0;
		let stream: TcpStream = loop {
			match TcpStream::connect((ip_addr, port)).await {
				Ok(s) => break s,
				Err(e) => {
					cont = cont.saturating_add(1);
					tokio::time::sleep(Duration::from_secs(1)).await;
					if cont >= 255 {
						return Err(format!("No se ha podido conectar con el servidor: {}", e).into());
					}
				}
			}
		};

		let (read_half, write_half) = stream.into_split();

		Ok(Client {
			name,
			reader: Mutex::new(BufReader::new(read_half)),
			writer: Mutex::new(write_half),
		})
	}

	pub async fn send_json(&self, req: Request) -> Result<(), Box<dyn Error + Send + Sync>> {
		let mut json_data: String = serde_json::to_string(&req)?;
		json_data.push('\n');

		println!(">Cliente> {}", json_data);

		let mut writer: tokio::sync::MutexGuard<'_, OwnedWriteHalf> = self.writer.lock().await;
		writer.write_all(json_data.as_bytes()).await?;
		writer.flush().await?;
		Ok(())
	}

	pub async fn recv_json(&self) -> Result<Response, Box<dyn Error + Send + Sync>> {
		let mut reader: tokio::sync::MutexGuard<'_, BufReader<OwnedReadHalf>> = self.reader.lock().await;
		let mut line: String = String::new();

		let n: usize = reader.read_line(&mut line).await?;
		if n == 0 {
			return Err("El servidor cerró la conexión".into());
		}

		let response: Response = serde_json::from_str(&line).map_err(|e| {
			println!("Error de deserialización: {}", e);
			println!("Contenido problemático: '{}'", line);
			e
		})?;

		println!("<Server< {}", serde_json::to_string(&response).unwrap_or_default());
		Ok(response)
	}
}

struct AppState {
	// Entender este tipo de dato es un poco raro. Lo podemos entender como si el profesor escribiese en una pizarra, echásemos un vistazo (Arc), modificásemos nosotros
	// Lo que hemos visto (Mutex), pero al entrar en nuestra memoria, comprobásemos si hemos visto letras, y si es así vamos en orden
	// Arc para acceder, Mutex para editar
	client: Arc<Mutex<Option<Arc<Client>>>>,
}

#[tauri::command]
async fn init(name: &str, state: State<'_, AppState>) -> Result<(), String> {

	let client: Client = Client::new(name.to_string(), "127.0.0.1".to_string(), 5005).await.map_err(|e| e.to_string())?;
	let client: Arc<Client> = Arc::new(client);

	let req: Request = Request {
		command: "init".to_string(),
		data: Some(client.name.clone()),
		status_code: true,
	};

	client.send_json(req).await.map_err(|e| e.to_string())?;
	let response: Response = client.recv_json().await.map_err(|e| e.to_string())?;

	if let Some(data) = response.data {
		if !data.is_empty() && data[0] == "Nombre repetido" {
			return Err("Ese nombre no está permitido.".into());
		}
	}

	// Guarda el Arc<Client> en el estado, esto causa breve lock
	{
		let mut guard: tokio::sync::MutexGuard<'_, Option<Arc<Client>>> = state.client.lock().await;
		*guard = Some(client.clone());
	}

	Ok(())
}

#[tauri::command]
async fn other_players(state: State<'_, AppState>) -> Result<Vec<String>, String> {

	// Cogemos rápidamente el Arc<Client> y soltamos el mutex del state. Al ser tan rápido no genera blockeos (presuntamente)
	let client_arc: Arc<Client> = {
		let guard: tokio::sync::MutexGuard<'_, Option<Arc<Client>>> = state.client.lock().await;
		guard.as_ref().cloned().ok_or("Cliente no inicializado".to_string())?
	};

	let req: Request = Request {
		command: "other_players".to_string(),
		data: None,
		status_code: true, 
	};

	client_arc.send_json(req).await.map_err(|e| e.to_string())?;
	let response: Response = client_arc.recv_json().await.map_err(|e| e.to_string())?; // Se elimina el client_arc
	Ok(response.data.unwrap_or_default())
}

#[tauri::command]
async fn request_party(name: &str, state: State<'_, AppState>) -> Result<bool, String> {

	let client_arc: Arc<Client> = {
		let guard: tokio::sync::MutexGuard<'_, Option<Arc<Client>>> = state.client.lock().await;
		guard.as_ref().cloned().ok_or("Cliente no inicializado".to_string())?
	};

	let req: Request = Request {
		command: "request_party".to_string(),
		data: Some(name.to_string()),
		status_code: true
	};

	client_arc.send_json(req).await.map_err(|e| e.to_string())?;
	let response: Response = client_arc.recv_json().await.map_err(|e| e.to_string())?;

	Ok(response.data.unwrap_or_default().get(0).map(|s| s == "party_accepted").unwrap_or(false))
}

fn plane_to_matrix(positions_plane: Vec<u8>) -> [[u8; 10]; 10] {

	println!("Longitud recibida: {}", positions_plane.len());

	let mut positions_matrix: [[u8; 10]; 10] = [[0; 10]; 10];
	for n in positions_plane {

		let n: usize = n as usize; // Es un poco raro redefinir la variable del bucle, pero mola. Debe ser usize para ser usado como índice. 32 o 64 bits
		positions_matrix[n / 10][n % 10] = 1 // row = x, col = y
	}

	positions_matrix
}

#[tauri::command]
async fn send_positions(positions: Vec<u8>, state: State<'_, AppState>) -> Result<bool, String> {

	println!("Invocado!");

	let client_arc: Option<Arc<Client>> = {
		let guard: tokio::sync::MutexGuard<'_, Option<Arc<Client>>> = state.client.lock().await;
		guard.as_ref().cloned()
	};

	let positions: [[u8; 10]; 10] = plane_to_matrix(positions);
	println!("{:?}", positions);
	Ok(true)
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
	tauri::Builder::default()
		.plugin(tauri_plugin_opener::init())
		.manage(AppState { 
			client: Arc::new(Mutex::new(None)) 
		}) 
		.invoke_handler(tauri::generate_handler![init, other_players, request_party, send_positions])
		.run(tauri::generate_context!())
		.expect("Error mientras se iniciaba la aplicación");
}
