use std::net::{Ipv4Addr, SocketAddrV4};
use tokio::io::{AsyncBufReadExt, BufReader, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex; // Usamos el Mutex de Tokio
use serde::{Serialize, Deserialize};
use std::time::Duration; // thread::sleep bloquea el runtime asíncrono, usaremos tokio::time
use std::error::Error;
use std::sync::Arc;
use tauri::State;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

// Estructuras auxiliares para los mensajes (El protocolo JSON)
#[derive(Serialize, Deserialize, Debug)]
struct Request {
	command: String,
	data: Option<String>, // Puede ser el nombre, u otro dato
	status_code: bool
}

// Respuesta para el servidor
#[derive(Serialize, Deserialize, Debug)]
struct Response {
	//status: Option<String>,
	command: String,
	data: Option<Vec<String>>,
	status_code: bool
}

struct Client {
	name: String,
	stream: BufReader<TcpStream>
}

impl Client {
	pub async fn new(name: String, ip: String, port: u16) -> Self {
		let ip_addr: Ipv4Addr = ip.parse().expect("IP inválida");
		println!("Conectando con {}:{}", ip_addr, port);

		let mut cont: u8 = 0;
		let stream: TcpStream = loop {
			match TcpStream::connect(SocketAddrV4::new(ip_addr, port)).await {
				Ok(stream) => break stream,
				Err(e) => {
					cont += 1;
					tokio::time::sleep(Duration::from_secs(1)).await;

					if cont >= 255 {
						panic!("No se ha podido conectar con el servidor: {}", e)
					}
				}
			}
		};

		Client {
			name: name,
			stream: BufReader::new(stream)
		}
	}

	async fn send_json(&mut self, req: Request) -> Result<(), Box<dyn Error>> {

		let mut json_data: String = serde_json::to_string(&req)?;
		json_data.push('\n');

		println!(">Cliente> {}", json_data);

		self.stream.write_all(json_data.as_bytes()).await?;
		self.stream.flush().await?;

		return Ok(());
	}

	async fn recv_json(&mut self) -> Result<Response, Box<dyn Error>> {
		let mut line: String = String::new();
		
		// read_line lee hasta el delimitador \n y lo guarda en el String
		let n: usize = self.stream.read_line(&mut line).await?;
		
		if n == 0 {
			return Err("El servidor cerró la conexión".into());
		}

		let response_result: Result<Response, _> = serde_json::from_str(&line);

		// Parseamos directamente desde el String
		let response: Response = match response_result {
			Ok(res) => res,
			Err(e) => {
				println!("Error de deserialización: {}", e);
				println!("Contenido problemático: '{}'", line);
				return Err(e.into());
			}
    	};
		
		match serde_json::to_string(&response) {
			Ok(pretty) => println!("<Server< {}", pretty),
			Err(e) => println!("Error al imprimir el JSON: {}", e),
    	}

		return Ok(response);
	}
}

struct AppState {
	// Arc<Mutex<T>> es el estándar para compartir estado mutable asíncrono
	client: Arc<Mutex<Option<Client>>>,
}

#[tauri::command]
async fn init(name: &str, state: State<'_, AppState>) -> Result<(), String> {
	let mut client: Client = Client::new(name.to_string(), "127.0.0.1".to_string(), 5005).await;

	let req: Request = Request {
		command: "init".to_string(),
		data: Some(client.name.clone()),
		status_code: true,
	};
	
	client.send_json(req).await.map_err(|e: Box<dyn Error>| e.to_string())?;
	
	let response: Response = client.recv_json().await.map_err(|e: Box<dyn Error>| e.to_string())?;

	if let Some(data) = response.data {
		if !data.is_empty() && data[0] == "Nombre repetido" {
			println!("Nombre duplicado en el servidor, {}", response.status_code.to_string());
			return Err("Ese nombre no está permitido. Se supone que este error no se debería poder ver".into());
		}
	}

	let mut drawer: tokio::sync::MutexGuard<'_, Option<Client>> = state.client.lock().await;
	*drawer = Some(client);

	return Ok(());
}

#[tauri::command]
async fn other_players(state: State<'_, AppState>) -> Result<Vec<String>, String> {
	let mut drawer: tokio::sync::MutexGuard<'_, Option<Client>> = state.client.lock().await;

	if let Some(client) = drawer.as_mut() {
		// Creamos la Request (adaptando tu lógica anterior a JSON)
		let req: Request = Request {
			command: "other_players".to_string(),
			data: None,
			status_code: true
		};

		client.send_json(req).await.map_err(|e: Box<dyn Error>| e.to_string())?;
		
		let response: Response = client.recv_json().await.map_err(|e: Box<dyn Error>| e.to_string())?;
		
		// Devolvemos la lista de jugadores si existe en la respuesta JSON
		return Ok(response.data.unwrap_or_default());
	} else {
		return Err("Cliente no inicializado".into());
	}
}

#[tauri::command]
async fn request_party(state: State<'_, AppState>, name: &str) -> Result<bool, String> {
	let mut drawer: tokio::sync::MutexGuard<'_, Option<Client>> = state.client.lock().await;

	if let Some(client) = drawer.as_mut() {
		let req: Request = Request {
			command: "request_party".to_string(),
			data: Some(name.to_string()),
			status_code: true
		};

		client.send_json(req).await.map_err(|e: Box<dyn Error>| e.to_string())?;
		
		let response: Response = client.recv_json().await.map_err(|e: Box<dyn Error>| e.to_string())?;

		if response.data.unwrap_or_default()[0] == "party_accepted" { 
			return Ok(true);
		}
		return Ok(false); // Habría que manejar en algún momento qué pasaría si la fiesta no inicia
	} else {
		return Err("Cliente no inicializado".into());
	}
	
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // Inicializamos el estado correctamente con la estructura
        .manage(AppState { 
            client: Arc::new(Mutex::new(None)) 
        }) 
        .invoke_handler(tauri::generate_handler![init, other_players, request_party])
        .run(tauri::generate_context!())
        .expect("Error mientras se iniciaba la aplicación");
}
