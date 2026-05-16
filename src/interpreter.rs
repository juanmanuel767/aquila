use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Condvar};
use crate::ast::{Expression, Param, Statement};
use std::fmt;
use pyo3::prelude::*;

fn ollama_generate_urls() -> Vec<String> {
    let mut urls = Vec::new();

    if let Ok(host) = std::env::var("OLLAMA_HOST") {
        let host = host.trim().trim_end_matches('/');
        if !host.is_empty() {
            let base = if host.starts_with("http://") || host.starts_with("https://") {
                host.to_string()
            } else {
                format!("http://{}", host)
            };
            urls.push(format!("{}/api/generate", base));
        }
    }

    urls.push("http://localhost:11434/api/generate".to_string());
    urls.push("http://127.0.0.1:11434/api/generate".to_string());
    urls.dedup();
    urls
}





#[derive(Debug, Clone, PartialEq)]
pub enum PromiseState {
    Pending,
    Resolved(Box<RuntimeValue>),
    Rejected(String),
}

#[derive(Debug, Clone)]
pub enum RuntimeValue {
    Number(f64),
    Int(i64),
    Boolean(bool),
    Text(String),
    List(Arc<Mutex<Vec<RuntimeValue>>>),
    Dictionary(Arc<Mutex<HashMap<String, RuntimeValue>>>),
    Function(Vec<Param>, Option<String>, Vec<Statement>),
    AsyncFunction(Vec<Param>, Option<String>, Vec<Statement>),
    Promise(Arc<(Mutex<PromiseState>, Condvar)>),
    Class(String, Option<String>, Arc<Mutex<HashMap<String, RuntimeValue>>>), // Nombre, Superclase?, Métodos
    Instance(String, Arc<Mutex<HashMap<String, RuntimeValue>>>, Box<RuntimeValue>),
    Server(Arc<crate::servidor::NeuroServer>),
    Database(Arc<crate::base_datos::AquilaDatabase>),
    PyWrapper(Arc<PyObject>),
    Null,
    Break,
    Continue,
}

// Implementación manual de PartialEq ignorando PyWrapper y Function porque no se pueden comparar directamente.
impl PartialEq for RuntimeValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (RuntimeValue::Number(a), RuntimeValue::Number(b)) => a == b,
            (RuntimeValue::Int(a), RuntimeValue::Int(b)) => a == b,
            (RuntimeValue::Boolean(a), RuntimeValue::Boolean(b)) => a == b,
            (RuntimeValue::Text(a), RuntimeValue::Text(b)) => a == b,
            (RuntimeValue::List(a), RuntimeValue::List(b)) => {
                let a_lock = a.lock().unwrap();
                let b_lock = b.lock().unwrap();
                *a_lock == *b_lock
            },
            (RuntimeValue::Dictionary(a), RuntimeValue::Dictionary(b)) => {
                let a_lock = a.lock().unwrap();
                let b_lock = b.lock().unwrap();
                *a_lock == *b_lock
            },
            (RuntimeValue::Class(n1, s1, _), RuntimeValue::Class(n2, s2, _)) => n1 == n2 && s1 == s2,
            (RuntimeValue::Instance(n1, p1, _), RuntimeValue::Instance(n2, p2, _)) => {
                let a_lock = p1.lock().unwrap();
                let b_lock = p2.lock().unwrap();
                n1 == n2 && *a_lock == *b_lock
            },
            (RuntimeValue::Promise(a), RuntimeValue::Promise(b)) => {
                let a_lock = a.0.lock().unwrap();
                let b_lock = b.0.lock().unwrap();
                *a_lock == *b_lock
            },
            (RuntimeValue::Server(_), RuntimeValue::Server(_)) => false,
            (RuntimeValue::Database(_), RuntimeValue::Database(_)) => false,
            (RuntimeValue::Null, RuntimeValue::Null) => true,
            (RuntimeValue::Break, RuntimeValue::Break) => true,
            (RuntimeValue::Continue, RuntimeValue::Continue) => true,
            _ => false,
        }
    }
}

impl fmt::Display for RuntimeValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeValue::Number(n) => write!(f, "{}", n),
            RuntimeValue::Int(i) => write!(f, "{}", i),
            RuntimeValue::Boolean(b) => write!(f, "{}", if *b { "verdadero" } else { "falso" }),
            RuntimeValue::Text(s) => write!(f, "{}", s),
            RuntimeValue::List(l) => {
                let l_lock = l.lock().unwrap();
                let elements: Vec<String> = l_lock.iter().map(|item| format!("{}", item)).collect();
                write!(f, "[{}]", elements.join(", "))
            },
            RuntimeValue::Dictionary(d) => {
                let d_lock = d.lock().unwrap();
                let elements: Vec<String> = d_lock.iter().map(|(k, v)| format!("\"{}\": {}", k, v)).collect();
                write!(f, "{{{}}}", elements.join(", "))
            },
            RuntimeValue::Class(name, super_name, _) => {
                if let Some(s) = super_name {
                    write!(f, "<clase {} hereda {}>", name, s)
                } else {
                    write!(f, "<clase {}>", name)
                }
            },
            RuntimeValue::Instance(name, props_arc, _) => {
                let props = props_arc.lock().unwrap();
                let elements: Vec<String> = props.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
                write!(f, "<instancia {}{{{}}}>", name, elements.join(", "))
            },
            RuntimeValue::Server(s) => write!(f, "<ServidorWeb puerto:{}>", s.port),
            RuntimeValue::Database(db) => write!(f, "<BaseDatos '{}'>", db.path),
            RuntimeValue::Function(params, _, _) => write!(f, "<funcion({})>", param_names(params).join(", ")),
            RuntimeValue::AsyncFunction(params, _, _) => write!(f, "<asincrono funcion({})>", param_names(params).join(", ")),
            RuntimeValue::Promise(p) => {
                let lock = p.0.lock().unwrap();
                match &*lock {
                    PromiseState::Pending => write!(f, "<promesa pendiente>"),
                    PromiseState::Resolved(v) => write!(f, "<promesa resuelta: {}>", v),
                    PromiseState::Rejected(e) => write!(f, "<promesa fallida: {}>", e),
                }
            },
            RuntimeValue::PyWrapper(_) => write!(f, "<Objeto Python>"),
            RuntimeValue::Null => write!(f, "nulo"),
            RuntimeValue::Break => write!(f, "<romper>"),
            RuntimeValue::Continue => write!(f, "<continuar>"),
        }
    }
}

// Conversores Univarsales
#[allow(deprecated)]
fn val_to_py(py: Python, val: RuntimeValue) -> PyObject {
    match val {
        RuntimeValue::Number(n) => n.to_object(py),
        RuntimeValue::Int(i) => i.to_object(py),
        RuntimeValue::Boolean(b) => b.to_object(py),
        RuntimeValue::Text(s) => s.to_object(py),
        RuntimeValue::Null => py.None(),
        RuntimeValue::List(l) => {
            let py_list = pyo3::types::PyList::empty(py);
            let l_lock = l.lock().unwrap();
            for item in l_lock.iter() {
                let _ = py_list.append(val_to_py(py, item.clone()));
            }
            py_list.to_object(py)
        },
        RuntimeValue::Dictionary(d) => {
            let py_dict = pyo3::types::PyDict::new(py);
            let d_lock = d.lock().unwrap();
            for (k, v) in d_lock.iter() {
                let _ = py_dict.set_item(k, val_to_py(py, v.clone()));
            }
            py_dict.to_object(py)
        },
        RuntimeValue::Class(_, _, _) => py.None(),
        RuntimeValue::Instance(_, _, _) => py.None(),
        RuntimeValue::Server(_) => py.None(),
        RuntimeValue::Database(_) => py.None(),
        RuntimeValue::PyWrapper(p) => (*p).clone_ref(py),
        _ => py.None()
    }
}

fn py_to_val(py: Python, py_obj: PyObject) -> RuntimeValue {
    if py_obj.is_none(py) { return RuntimeValue::Null; }
    if let Ok(b) = py_obj.extract::<bool>(py) { return RuntimeValue::Boolean(b); }
    if let Ok(n) = py_obj.extract::<i64>(py) { return RuntimeValue::Int(n); }
    if let Ok(n) = py_obj.extract::<f64>(py) { return RuntimeValue::Number(n); }
    if let Ok(s) = py_obj.extract::<String>(py) { return RuntimeValue::Text(s); }
    
    // Conversión recursiva de Listas/Tuplas
    if let Ok(list) = py_obj.bind(py).downcast::<pyo3::types::PyList>() {
        let mut items = Vec::new();
        for item in list.iter() {
            items.push(py_to_val(py, item.to_object(py)));
        }
        return RuntimeValue::List(Arc::new(Mutex::new(items)));
    }
    if let Ok(tuple) = py_obj.bind(py).downcast::<pyo3::types::PyTuple>() {
        let mut items = Vec::new();
        for item in tuple.iter() {
            items.push(py_to_val(py, item.to_object(py)));
        }
        return RuntimeValue::List(Arc::new(Mutex::new(items)));
    }
    
    // Conversión recursiva de Diccionarios
    if let Ok(dict) = py_obj.bind(py).downcast::<pyo3::types::PyDict>() {
        let mut map = HashMap::new();
        for (k, v) in dict.iter() {
            if let Ok(key_str) = k.extract::<String>() {
                map.insert(key_str, py_to_val(py, v.to_object(py)));
            }
        }
        return RuntimeValue::Dictionary(Arc::new(Mutex::new(map)));
    }
    
    RuntimeValue::PyWrapper(Arc::new(py_obj))
}

fn runtime_to_json_value(val: &RuntimeValue) -> Result<serde_json::Value, String> {
    match val {
        RuntimeValue::Number(n) => Ok(serde_json::json!(n)),
        RuntimeValue::Int(i) => Ok(serde_json::json!(i)),
        RuntimeValue::Boolean(b) => Ok(serde_json::json!(b)),
        RuntimeValue::Text(s) => Ok(serde_json::json!(s)),
        RuntimeValue::Null => Ok(serde_json::Value::Null),
        RuntimeValue::List(items_arc) => {
            let items = items_arc.lock().unwrap();
            let mut json_items = Vec::new();
            for item in items.iter() {
                json_items.push(runtime_to_json_value(item)?);
            }
            Ok(serde_json::Value::Array(json_items))
        },
        RuntimeValue::Dictionary(map_arc) => {
            let map = map_arc.lock().unwrap();
            let mut json_map = serde_json::Map::new();
            for (key, value) in map.iter() {
                json_map.insert(key.clone(), runtime_to_json_value(value)?);
            }
            Ok(serde_json::Value::Object(json_map))
        },
        _ => Err(format!("Este valor no se puede convertir a JSON: {}", val)),
    }
}

fn normalize_type_name(type_name: &str) -> String {
    type_name.trim().to_lowercase()
}

fn runtime_type_name(value: &RuntimeValue) -> &'static str {
    match value {
        RuntimeValue::Int(_) => "Entero",
        RuntimeValue::Number(_) => "Decimal",
        RuntimeValue::Boolean(_) => "Booleano",
        RuntimeValue::Text(_) => "Texto",
        RuntimeValue::List(_) => "Lista",
        RuntimeValue::Dictionary(_) => "Diccionario",
        RuntimeValue::Null => "Nulo",
        RuntimeValue::Function(_, _, _) => "Funcion",
        RuntimeValue::AsyncFunction(_, _, _) => "FuncionAsincrona",
        RuntimeValue::Promise(_) => "Promesa",
        RuntimeValue::Class(_, _, _) => "Clase",
        RuntimeValue::Instance(_, _, _) => "Instancia",
        RuntimeValue::Server(_) => "Servidor",
        RuntimeValue::Database(_) => "BaseDatos",
        RuntimeValue::PyWrapper(_) => "Python",
        RuntimeValue::Break => "Romper",
        RuntimeValue::Continue => "Continuar",
    }
}

fn validate_runtime_type(value: &RuntimeValue, type_name: &str) -> Result<(), String> {
    let normalized = normalize_type_name(type_name);
    let ok = match normalized.as_str() {
        "cualquiera" | "any" => true,
        "entero" | "int" => matches!(value, RuntimeValue::Int(_)),
        "decimal" | "numero" | "number" => matches!(value, RuntimeValue::Number(_) | RuntimeValue::Int(_)),
        "texto" | "string" => matches!(value, RuntimeValue::Text(_)),
        "booleano" | "bool" => matches!(value, RuntimeValue::Boolean(_)),
        "lista" | "list" => matches!(value, RuntimeValue::List(_)),
        "diccionario" | "dict" => matches!(value, RuntimeValue::Dictionary(_)),
        "nulo" | "null" => matches!(value, RuntimeValue::Null),
        "funcion" => matches!(value, RuntimeValue::Function(_, _, _)),
        "funcionasincrona" | "asincrono" => matches!(value, RuntimeValue::AsyncFunction(_, _, _)),
        "servidor" => matches!(value, RuntimeValue::Server(_)),
        "basedatos" => matches!(value, RuntimeValue::Database(_)),
        "python" => matches!(value, RuntimeValue::PyWrapper(_)),
        _ => {
            return Err(format!(
                "Tipo desconocido '{}'. Usa Entero, Decimal, Texto, Booleano, Lista, Diccionario, Nulo o Cualquiera.",
                type_name
            ));
        },
    };

    if ok {
        Ok(())
    } else {
        Err(format!(
            "La variable esperaba tipo {}, pero recibió {}.",
            type_name,
            runtime_type_name(value)
        ))
    }
}

fn param_names(params: &[Param]) -> Vec<String> {
    params.iter().map(|param| param.name.clone()).collect()
}

fn bind_params(
    params: &[Param],
    eval_args: &[RuntimeValue],
    call_env: &Arc<Mutex<Environment>>,
) -> Result<(), String> {
    for (i, param) in params.iter().enumerate() {
        let value = eval_args[i].clone();
        if let Some(type_name) = &param.type_name {
            call_env.lock().unwrap().define_typed(param.name.clone(), type_name.clone(), value)?;
        } else {
            call_env.lock().unwrap().define(param.name.clone(), value);
        }
    }
    Ok(())
}

pub struct Environment {
    values: HashMap<String, RuntimeValue>,
    types: HashMap<String, String>,
    observers: HashMap<String, Vec<Vec<Statement>>>,
    reactive_vars: HashSet<String>,
    parent: Option<Arc<Mutex<Environment>>>,
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            values: HashMap::new(),
            types: HashMap::new(),
            observers: HashMap::new(),
            reactive_vars: HashSet::new(),
            parent: None,
        }
    }

    pub fn new_with_parent(parent: Arc<Mutex<Environment>>) -> Self {
        Environment {
            values: HashMap::new(),
            types: HashMap::new(),
            observers: HashMap::new(),
            reactive_vars: HashSet::new(),
            parent: Some(parent),
        }
    }

    pub fn define(&mut self, name: String, value: RuntimeValue) {
        self.values.insert(name, value);
    }

    pub fn define_typed(&mut self, name: String, type_name: String, value: RuntimeValue) -> Result<(), String> {
        validate_runtime_type(&value, &type_name)?;
        self.values.insert(name.clone(), value);
        self.types.insert(name, type_name);
        Ok(())
    }

    pub fn define_reactive(&mut self, name: String, value: RuntimeValue) {
        self.values.insert(name.clone(), value);
        self.reactive_vars.insert(name);
    }

    pub fn add_observer(&mut self, name: String, body: Vec<Statement>) {
        if self.reactive_vars.contains(&name) {
            self.observers.entry(name).or_insert_with(Vec::new).push(body);
        } else if let Some(p) = &self.parent {
            p.lock().unwrap().add_observer(name, body);
        }
    }

    pub fn get_observers(&self, name: &str) -> Vec<Vec<Statement>> {
        let mut obs = self.observers.get(name).cloned().unwrap_or_default();
        if let Some(p) = &self.parent {
            obs.extend(p.lock().unwrap().get_observers(name));
        }
        obs
    }

    pub fn assign(&mut self, name: String, value: RuntimeValue) -> Result<bool, String> {
        if self.values.contains_key(&name) {
            if let Some(type_name) = self.types.get(&name) {
                validate_runtime_type(&value, type_name)?;
            }
            self.values.insert(name.clone(), value);
            Ok(self.reactive_vars.contains(&name))
        } else if let Some(parent) = &mut self.parent {
            parent.lock().unwrap().assign(name, value)
        } else {
            Err(format!("Variable no definida: {}", name))
        }
    }

    pub fn get(&self, name: &str) -> Result<RuntimeValue, String> {
        if let Some(val) = self.values.get(name) {
            Ok(val.clone())
        } else if let Some(parent) = &self.parent {
            parent.lock().unwrap().get(name)
        } else {
            Err(format!("Variable no definida: '{}'", name))
        }
    }
}

#[derive(Clone)]
pub struct Interpreter {
    pub global_env: Arc<Mutex<Environment>>,
    pub exported_names: Vec<String>,
    pub base_dir: PathBuf,
    pub resolver: crate::depredactor::DepredactorResolver,
    pub output_buffer: Option<Arc<Mutex<Vec<String>>>>,
}

impl Interpreter {
    pub fn new() -> Self {
        Self::with_base_dir(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }

    pub fn with_base_dir(base_dir: PathBuf) -> Self {
        let env = Arc::new(Mutex::new(Environment::new()));
        let resolver = crate::depredactor::DepredactorResolver::new(base_dir.clone());
        setup_globals(&env);
        Interpreter {
            global_env: env,
            exported_names: Vec::new(),
            resolver,
            base_dir,
            output_buffer: None,
        }
    }



    fn python_dependency_hint(&self, module: &str) -> String {
        let root_package = module.split('.').next().unwrap_or(module);
        match self.resolver.check_manifest(root_package, self) {
            Some(origin) if origin == "python" => format!(
                "\n💡 Esta dependencia está registrada en neurocode.json pero no instalada.\n   Ejecuta: neuro instalar python:{}",
                root_package
            ),
            Some(origin) => format!(
                "\n💡 '{}' aparece en neurocode.json con origen '{}', no como Python. Revisa el prefijo de depredactor o registra: neuro instalar python:{}",
                root_package, origin, root_package
            ),
            None => format!(
                "\n💡 Esta dependencia no está registrada en neurocode.json.\n   Ejecuta: neuro instalar {}",
                root_package
            ),
        }
    }

    pub fn interpret(&mut self, statements: Vec<Statement>) -> Result<RuntimeValue, String> {
        // Inicializamos Python para toda la sesión de Nexus (Fase 4)
        pyo3::prepare_freethreaded_python();
        
        let env = Arc::clone(&self.global_env);
        for stmt in statements {
            if let Some(ret) = self.execute(stmt, &env)? {
                return Ok(ret);
            }
        }
        Ok(RuntimeValue::Null)
    }

    fn execute(&mut self, stmt: Statement, env: &Arc<Mutex<Environment>>) -> Result<Option<RuntimeValue>, String> {
        match stmt {
            Statement::Assign(name, expr) => {
                let value = self.evaluate(expr, env)?;
                let observers = {
                    let mut env_lock = env.lock().unwrap();
                    match env_lock.assign(name.clone(), value.clone()) {
                        Ok(is_reactive) => {
                            if is_reactive { env_lock.get_observers(&name) } else { vec![] }
                        },
                        Err(_) => {
                            env_lock.define(name, value);
                            vec![]
                        }
                    }
                };
                for obs in observers {
                    self.execute_block(obs, env)?;
                }
            },
            Statement::AssignTyped(name, type_name, expr) => {
                let value = self.evaluate(expr, env)?;
                env.lock().unwrap().define_typed(name, type_name, value)?;
            },
            Statement::AssignProperty(callee_expr, prop_name, value_expr) => {
                let callee = self.evaluate(callee_expr, env)?;
                let value = self.evaluate(value_expr, env)?;
                match callee {
                    RuntimeValue::Dictionary(map_arc) => {
                        map_arc.lock().unwrap().insert(prop_name, value);
                    },
                    RuntimeValue::Instance(_, props_arc, _) => {
                        props_arc.lock().unwrap().insert(prop_name, value);
                    },
                    RuntimeValue::PyWrapper(py_obj) => {
                        pyo3::Python::with_gil(|py| {
                            let _ = py_obj.setattr(py, prop_name.as_str(), val_to_py(py, value));
                        });
                    },
                    _ => return Err("Solo se pueden asignar propiedades a diccionarios u objetos de Python.".into()),
                }
            },
            Statement::AssignIndex(callee_expr, index_expr, value_expr) => {
                let callee = self.evaluate(callee_expr, env)?;
                let index = self.evaluate(index_expr, env)?;
                let value = self.evaluate(value_expr, env)?;
                
                match callee {
                    RuntimeValue::List(list_arc) => {
                        if let RuntimeValue::Int(idx) = index {
                            let mut list = list_arc.lock().unwrap();
                            if idx >= 0 && (idx as usize) < list.len() {
                                list[idx as usize] = value;
                            } else {
                                return Err(format!("Índice de lista fuera de límites: {}", idx));
                            }
                        } else {
                            return Err("El índice de una lista debe ser entero.".into());
                        }
                    },
                    RuntimeValue::Dictionary(map_arc) => {
                        let str_key = match index {
                            RuntimeValue::Text(s) => s,
                            _ => return Err("El índice de diccionarios al asignar debe ser texto.".into()),
                        };
                        map_arc.lock().unwrap().insert(str_key, value);
                    },
                    _ => return Err("Solo se puede asignar por índice a listas o diccionarios.".into()),
                }
            },
            Statement::Expression(expr) => {
                self.evaluate(expr, env)?;
            },
            Statement::If(cond, then_branch, else_branch) => {
                let cond_val = self.evaluate(cond, env)?;
                if self.is_truthy(&cond_val) {
                    if let Some(ret) = self.execute_block(then_branch, env)? { return Ok(Some(ret)); }
                } else {
                    if let Some(ret) = self.execute_block(else_branch, env)? { return Ok(Some(ret)); }
                }
            },
            Statement::While(cond, body) => {
                loop {
                    let cond_val = self.evaluate(cond.clone(), env)?;
                    if !self.is_truthy(&cond_val) { break; }
                    if let Some(ret) = self.execute_block(body.clone(), env)? {
                        if let RuntimeValue::Break = ret { break; }
                        if let RuntimeValue::Continue = ret { continue; }
                        return Ok(Some(ret));
                    }
                }
            },
            Statement::For(var_name, iterable_expr, body) => {
                let iterable_val = self.evaluate(iterable_expr, env)?;
                if let RuntimeValue::List(list_arc) = iterable_val {
                    let list = list_arc.lock().unwrap().clone();
                    for item in list {
                        let local_env = Arc::new(Mutex::new(Environment::new_with_parent(Arc::clone(env))));
                        local_env.lock().unwrap().define(var_name.clone(), item);
                        if let Some(ret) = self.execute_block(body.clone(), &local_env)? {
                            if let RuntimeValue::Break = ret { break; }
                            if let RuntimeValue::Continue = ret { continue; }
                            return Ok(Some(ret));
                        }
                    }
                } else {
                    return Err("El bucle 'para' solo soporta iterar sobre listas u objetos iterables.".into());
                }
            },
            Statement::Break => {
                return Ok(Some(RuntimeValue::Break));
            },
            Statement::Continue => {
                return Ok(Some(RuntimeValue::Continue));
            },
            Statement::Function(name, params, return_type, body) => {
                env.lock().unwrap().define(name, RuntimeValue::Function(params, return_type, body));
            },
            Statement::AsyncFunction(name, params, return_type, body) => {
                env.lock().unwrap().define(name, RuntimeValue::AsyncFunction(params, return_type, body));
            },
            Statement::Return(expr) => {
                return Ok(Some(self.evaluate(expr, env)?));
            },
            Statement::Export(inner) => {
                self.execute(*inner.clone(), env)?;
                match *inner {
                    Statement::Assign(name, _) |
                    Statement::AssignTyped(name, _, _) |
                    Statement::Function(name, _, _, _) |
                    Statement::AsyncFunction(name, _, _, _) |
                    Statement::Class(name, _, _) => {
                        self.exported_names.push(name);
                    },
                    _ => {}
                }
            },
             Statement::Usar(modulo, alias) => {
                 let source = self.resolver.resolve(&modulo, self);

                 if let crate::depredactor::ModuleSource::Python(py_module_name) = &source {
                     let py_module = pyo3::Python::with_gil(|py| {
                         match py.import(py_module_name.as_str()) {
                             Ok(m) => {
                                 #[allow(deprecated)]
                                 let obj = m.to_object(py);
                                 Ok(RuntimeValue::PyWrapper(Arc::new(obj)))
                             },
                             Err(e) => {
                                 e.print(py);
                                 Err(format!(
                                     "Depredactor no pudo cazar la librería Python '{}'. {}",
                                     py_module_name,
                                     self.python_dependency_hint(py_module_name)
                                 ))
                             },
                         }
                     })?;
                     env.lock().unwrap().define(alias, py_module);
                     return Ok(None);
                 }

                 if let crate::depredactor::ModuleSource::NotFound(message) = &source {
                     return Err(message.clone());
                 }

                 let content = match &source {
                     crate::depredactor::ModuleSource::Remote(remote_url) => {
                         let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_else(|_| ".".to_string());
                         let cache_dir = format!("{}/.aquila_cache", home);
                         let _ = std::fs::create_dir_all(&cache_dir);

                         let safe_name = remote_url.replace("://", "_").replace("/", "_").replace(":", "_");
                         let cache_path = format!("{}/{}", cache_dir, safe_name);

                         if let Ok(cached) = std::fs::read_to_string(&cache_path) {
                             cached
                         } else {
                             println!("{}☁️ Guardián de Paquetes: Descargando módulo remoto: {}...{}", "\x1b[36m", remote_url, "\x1b[0m");
                             match ureq::get(remote_url).call() {
                                 Ok(resp) => {
                                     let text = resp.into_string().map_err(|e| format!("Error leyendo respuesta: {}", e))?;
                                     let _ = std::fs::write(&cache_path, &text);
                                     text
                                 },
                                 Err(e) => return Err(format!("No se pudo descargar el módulo remoto '{}': {}", remote_url, e)),
                             }
                         }
                     },
                     crate::depredactor::ModuleSource::NeuroFile(module_path) => {
                         std::fs::read_to_string(module_path)
                             .map_err(|e| format!("No se pudo leer el archivo módulo '{}': {}", module_path.display(), e))?
                     },
                     crate::depredactor::ModuleSource::Python(_) => unreachable!(),
                     crate::depredactor::ModuleSource::Remote(_) => unreachable!(),
                     crate::depredactor::ModuleSource::System(target) => {
                         return Err(format!("El módulo del sistema '{}' no se puede cargar directamente. Usa comandos de sistema.", target));
                     }
                     crate::depredactor::ModuleSource::NotFound(_) => unreachable!(),
                 };

                     let tokens = crate::lexer::tokenize(&content);
                     let statements = match crate::parser::parse(tokens) {
                          Ok(s) => s,
                          Err(e) => return Err(format!("Error parseando módulo '{}': {}", modulo, e))
                     };
                     // Prevenir conflictos de entorno al importar creando una nueva instancia limpia de Intérprete
                     let module_base_dir = match &source {
                          crate::depredactor::ModuleSource::Remote(_) => self.base_dir.clone(),
                          crate::depredactor::ModuleSource::NeuroFile(module_path) => module_path
                              .parent()
                              .map(|p| p.to_path_buf())
                              .unwrap_or_else(|| self.base_dir.clone()),
                          _ => self.base_dir.clone(),
                     };
                     let mut mod_interp = Interpreter::with_base_dir(module_base_dir);
                     let _ = mod_interp.interpret(statements)?;
                     
                     let mut exported_map = HashMap::new();
                     let mod_env = mod_interp.global_env.lock().unwrap();
                     for exp_name in mod_interp.exported_names {
                         if let Ok(val) = mod_env.get(&exp_name) {
                             exported_map.insert(exp_name, val);
                         }
                     }
                     
                     let module_dict = RuntimeValue::Dictionary(Arc::new(Mutex::new(exported_map)));
                     env.lock().unwrap().define(alias, module_dict);
             },
             Statement::TryCatch(try_block, error_var, catch_block) => {
                 match self.execute_block(try_block, env) {
                     Ok(opt_val) => {
                         if opt_val.is_some() { return Ok(opt_val); }
                     },
                     Err(e) => {
                         let catch_env = Arc::new(Mutex::new(Environment::new_with_parent(Arc::clone(env))));
                         if let Some(var_name) = error_var {
                             catch_env.lock().unwrap().define(var_name.clone(), RuntimeValue::Text(e));
                         }
                         if let Some(val) = self.execute_block(catch_block, &catch_env)? {
                             return Ok(Some(val));
                         }
                     }
                 }
             },
             Statement::Throw(expr) => {
                 let val = self.evaluate(expr, env)?;
                 return Err(format!("{}", val));
             },
             Statement::Class(name, super_class, methods) => {
                let mut class_methods = HashMap::new();
                for method in methods {
                    if let Statement::Function(m_name, params, ret, body) = method {
                        class_methods.insert(m_name, RuntimeValue::Function(params, ret, body));
                    } else if let Statement::AsyncFunction(m_name, params, ret, body) = method {
                        class_methods.insert(m_name, RuntimeValue::AsyncFunction(params, ret, body));
                    }
                }
                env.lock().unwrap().define(name.clone(), RuntimeValue::Class(name, super_class, Arc::new(Mutex::new(class_methods))));
            },
             Statement::Parallel(stmts) => {
                 let mut handles = Vec::new();
                 for stmt in stmts {
                     if let Statement::Task(task_stmt) = stmt {
                         let mut interpreter_thread = self.clone();
                         let env_thread = Arc::clone(env);
                         let handle = std::thread::spawn(move || {
                             let _ = interpreter_thread.execute(*task_stmt, &env_thread);
                         });
                         handles.push(handle);
                     } else {
                         if let Some(ret) = self.execute(stmt, env)? {
                             return Ok(Some(ret));
                         }
                     }
                 }
                 for handle in handles {
                     let _ = handle.join();
                 }
             },
             Statement::Task(stmt) => {
                let mut interpreter_thread = self.clone();
                let env_thread = Arc::clone(env);
                std::thread::spawn(move || {
                    let _ = interpreter_thread.execute(*stmt, &env_thread);
                });
            },
            Statement::Block(stmts) => {
                return self.execute_block(stmts, env);
            },
            Statement::Reactive(name, expr) => {
                let val = self.evaluate(expr, env)?;
                env.lock().unwrap().define_reactive(name, val);
            },
            Statement::ReactObserve(name, body) => {
                env.lock().unwrap().add_observer(name, body);
            },
            Statement::Api(routes) => {
                let server = crate::servidor::NeuroServer::new(8080); // Puerto por defecto
                for route in routes {
                    if let Statement::ApiRoute(path, body) = route {
                        server.add_route("GET".to_string(), path, vec![], body);
                    }
                }
                let server_arc = Arc::new(server);
                server_arc.start(self, env)?;
            },
            _ => {}
        }
        Ok(None)
    }

    pub fn execute_block_pub(&mut self, statements: Vec<Statement>, env: &Arc<Mutex<Environment>>) -> Result<Option<RuntimeValue>, String> {
        self.execute_block(statements, env)
    }

    fn execute_block(&mut self, statements: Vec<Statement>, env: &Arc<Mutex<Environment>>) -> Result<Option<RuntimeValue>, String> {
        let block_env = Arc::new(Mutex::new(Environment::new_with_parent(Arc::clone(env))));
        for stmt in statements {
            if let Some(ret) = self.execute(stmt, &block_env)? {
                return Ok(Some(ret));
            }
        }
        Ok(None)
    }

    fn evaluate(&mut self, expr: Expression, env: &Arc<Mutex<Environment>>) -> Result<RuntimeValue, String> {
        match expr {
            Expression::Number(n) => Ok(RuntimeValue::Number(n)),
            Expression::Int(i) => Ok(RuntimeValue::Int(i)),
            Expression::Text(s) => Ok(RuntimeValue::Text(s)),
            Expression::Boolean(b) => Ok(RuntimeValue::Boolean(b)),
            Expression::Null => Ok(RuntimeValue::Null),
            Expression::List(items) => {
                let mut eval_items = Vec::new();
                for item in items {
                    eval_items.push(self.evaluate(item, env)?);
                }
                Ok(RuntimeValue::List(Arc::new(Mutex::new(eval_items))))
            },
            Expression::Dictionary(pairs) => {
                let mut map = HashMap::new();
                for (k, v) in pairs {
                    let key_val = self.evaluate(k, env)?;
                    let str_key = match key_val {
                        RuntimeValue::Text(s) => s,
                        _ => return Err("Las claves de diccionarios deben ser texto.".into()),
                    };
                    let val_val = self.evaluate(v, env)?;
                    map.insert(str_key, val_val);
                }
                Ok(RuntimeValue::Dictionary(Arc::new(Mutex::new(map))))
            },
            Expression::NewInstance(class_name, args) => {
                // ServidorWeb nativo
                if class_name == "ServidorWeb" {
                    if let Some(port_val) = args.get(0) {
                        let port = match self.evaluate(port_val.clone(), env)? {
                            RuntimeValue::Int(p) => p as u16,
                            RuntimeValue::Number(p) => p as u16,
                            _ => return Err("ServidorWeb() requiere un número de puerto.".into()),
                        };
                        return Ok(RuntimeValue::Server(Arc::new(crate::servidor::NeuroServer::new(port))));
                    }
                    return Err("ServidorWeb() requiere 1 argumento (puerto).".into());
                }
                
                // BaseDatos nativa
                if class_name == "BaseDatos" {
                    if let Some(path_val) = args.get(0) {
                        let path = match self.evaluate(path_val.clone(), env)? {
                            RuntimeValue::Text(p) => p,
                            _ => return Err("BaseDatos() requiere una ruta de archivo como texto.".into()),
                        };
                        match crate::base_datos::AquilaDatabase::new(&path) {
                            Ok(db) => return Ok(RuntimeValue::Database(Arc::new(db))),
                            Err(e) => return Err(e),
                        }
                    }
                    return Err("BaseDatos() requiere 1 argumento (ruta).".into());
                }
                
                let class_val_res = env.lock().unwrap().get(&class_name);
                if let Ok(RuntimeValue::Class(name, super_class, methods_arc)) = class_val_res {
                    let instance_props = Arc::new(Mutex::new(HashMap::new()));
                    let instance_val = RuntimeValue::Instance(name.clone(), instance_props, Box::new(RuntimeValue::Class(name, super_class, methods_arc.clone())));
                    
                    let methods = methods_arc.lock().unwrap();
                    if let Some(RuntimeValue::Function(params, _, body)) = methods.get("crear") {
                        if args.len() != params.len() {
                            return Err(format!("El constructor 'crear' de '{}' esperaba {} argumentos pero recibió {}.", class_name, params.len(), args.len()));
                        }
                        
                        let call_env = Arc::new(Mutex::new(Environment::new_with_parent(Arc::clone(env))));
                        call_env.lock().unwrap().define("esto".to_string(), instance_val.clone());
                        
                        let mut constructor_args = Vec::new();
                        for arg in args.iter().take(params.len()) {
                            constructor_args.push(self.evaluate(arg.clone(), env)?);
                        }
                        bind_params(params, &constructor_args, &call_env)?;
                        self.execute_block(body.clone(), &call_env)?;
                    } else if !args.is_empty() {
                        return Err(format!("La clase '{}' no tiene constructor 'crear' pero se recibieron argumentos.", class_name));
                    }
                    
                    return Ok(instance_val);
                }
                Err(format!("Clase invocada '{}' no encontrada en el contexto.", class_name))
            },
            Expression::Await(expr) => {
                let val = self.evaluate(*expr, env)?;
                if let RuntimeValue::Promise(p) = val {
                    let (lock, cvar) = &*p;
                    let mut state = lock.lock().unwrap();
                    while let PromiseState::Pending = *state {
                        state = cvar.wait(state).unwrap();
                    }
                    match &*state {
                        PromiseState::Resolved(v) => Ok((**v).clone()),
                        PromiseState::Rejected(e) => Err(e.clone()),
                        PromiseState::Pending => unreachable!(),
                    }
                } else {
                    Ok(val)
                }
            },
            Expression::LambdaFunction(params, return_type, body) => {
                Ok(RuntimeValue::Function(params, return_type, body))
            },
            Expression::Identifier(name) => {
                env.lock().unwrap().get(&name)
            },
            Expression::LogicalOp(left_expr, op, right_expr) => {
                let l_val = self.evaluate(*left_expr, env)?;
                if op == "o" {
                    if self.is_truthy(&l_val) { return Ok(l_val); }
                } else if op == "y" {
                    if !self.is_truthy(&l_val) { return Ok(l_val); }
                }
                self.evaluate(*right_expr, env)
            },
            Expression::BinaryOp(left, op, right) => {
                let l_val = self.evaluate(*left, env)?;
                let r_val = self.evaluate(*right, env)?;
                self.evaluate_binary(l_val, &op, r_val)
            },
            Expression::UnaryOp(op, right) => {
                let r_val = self.evaluate(*right, env)?;
                self.evaluate_unary(&op, r_val)
            },
            Expression::FunctionCall(name, args) => {
                let mut eval_args = Vec::new();
                for arg in &args {
                    eval_args.push(self.evaluate(arg.clone(), env)?);
                }
                
                if name == "imprimir" || name == "mostrar" {
                    let out_str: Vec<String> = eval_args.iter().map(|a| format!("{}", a)).collect();
                    if let Some(buf) = &self.output_buffer {
                        buf.lock().unwrap().push(out_str.join(" "));
                    } else {
                        println!("{}", out_str.join(" "));
                    }
                    return Ok(RuntimeValue::Null);
                }
                
                if name == "rango" {
                    if let Some(RuntimeValue::Number(n)) = eval_args.get(0) {
                        let items = (0..(*n as i64)).map(|i| RuntimeValue::Int(i)).collect();
                        return Ok(RuntimeValue::List(Arc::new(Mutex::new(items))));
                    }
                    if let Some(RuntimeValue::Int(n)) = eval_args.get(0) {
                        let items = (0..*n).map(|i| RuntimeValue::Int(i)).collect();
                        return Ok(RuntimeValue::List(Arc::new(Mutex::new(items))));
                    }
                }
                
                if name == "tipo" {
                    if let Some(arg) = eval_args.get(0) {
                        let t = match arg {
                            RuntimeValue::Number(_) => "numero",
                            RuntimeValue::Int(_) => "entero",
                            RuntimeValue::Boolean(_) => "booleano",
                            RuntimeValue::Text(_) => "texto",
                            RuntimeValue::List(_) => "lista",
                            RuntimeValue::Dictionary(_) => "diccionario",
                            RuntimeValue::Function(_, _, _) => "funcion",
                            RuntimeValue::AsyncFunction(_, _, _) => "funcion_asincrona",
                            RuntimeValue::Promise(_) => "promesa",
                            RuntimeValue::Class(_, _, _) => "clase",
                            RuntimeValue::Instance(_, _, _) => "instancia",
                            RuntimeValue::Server(_) => "servidor",
                            RuntimeValue::Database(_) => "base_datos",
                            RuntimeValue::PyWrapper(_) => "python",
                            RuntimeValue::Null => "nulo",
                            RuntimeValue::Break => "romper",
                            RuntimeValue::Continue => "continuar",
                        };
                        return Ok(RuntimeValue::Text(t.to_string()));
                    }
                    return Err("tipo() requiere 1 argumento.".into());
                }

                if name == "longitud" {
                    if let Some(arg) = eval_args.get(0) {
                        match arg {
                            RuntimeValue::Text(s) => return Ok(RuntimeValue::Int(s.len() as i64)),
                            RuntimeValue::List(l_arc) => return Ok(RuntimeValue::Int(l_arc.lock().unwrap().len() as i64)),
                            _ => return Err("longitud() solo aplica a texto o listas.".into())
                        }
                    }
                    return Err("longitud() requiere 1 argumento.".into());
                }

                if name == "entrada" {
                    use std::io::{self, Write};
                    if let Some(msg) = eval_args.get(0) {
                        print!("{}", msg);
                    }
                    io::stdout().flush().unwrap();
                    let mut input = String::new();
                    if io::stdin().read_line(&mut input).is_ok() {
                        return Ok(RuntimeValue::Text(input.trim_end().to_string()));
                    }
                    return Ok(RuntimeValue::Text("".to_string()));
                }

                if name == "a_numero" {
                    if let Some(val) = eval_args.get(0) {
                        return match val {
                            RuntimeValue::Text(s) => {
                                if let Ok(i) = s.parse::<i64>() { Ok(RuntimeValue::Int(i)) }
                                else if let Ok(n) = s.parse::<f64>() { Ok(RuntimeValue::Number(n)) }
                                else { Err(format!("No se puede convertir '{}' a número.", s)) }
                            },
                            RuntimeValue::Int(_) | RuntimeValue::Number(_) => Ok(val.clone()),
                            _ => Err("a_numero() requiere texto o número.".into()),
                        };
                    }
                }

                if name == "a_texto" {
                    if let Some(val) = eval_args.get(0) {
                        return Ok(RuntimeValue::Text(format!("{}", val)));
                    }
                }

                if name == "entero" {
                    if let Some(arg) = eval_args.get(0) {
                        match arg {
                            RuntimeValue::Text(s) => {
                                if let Ok(n) = s.parse::<i64>() { return Ok(RuntimeValue::Int(n)); }
                                return Err(format!("No se puede convertir '{}' a entero.", s));
                            },
                            RuntimeValue::Number(n) => return Ok(RuntimeValue::Int(*n as i64)),
                            RuntimeValue::Int(n) => return Ok(RuntimeValue::Int(*n)),
                            _ => return Err("entero() requiere un texto numérico o número.".into())
                        }
                    }
                    return Err("entero() requiere 1 argumento.".into());
                }
                
                if name == "decimal" {
                    if let Some(arg) = eval_args.get(0) {
                        match arg {
                            RuntimeValue::Text(s) => {
                                if let Ok(n) = s.parse::<f64>() { return Ok(RuntimeValue::Number(n)); }
                                return Err(format!("No se puede convertir '{}' a decimal.", s));
                            },
                            RuntimeValue::Int(n) => return Ok(RuntimeValue::Number(*n as f64)),
                            RuntimeValue::Number(n) => return Ok(RuntimeValue::Number(*n)),
                            _ => return Err("decimal() requiere un texto numérico o número.".into())
                        }
                    }
                    return Err("decimal() requiere 1 argumento.".into());
                }

                if name == "texto" {
                    if let Some(arg) = eval_args.get(0) {
                        return Ok(RuntimeValue::Text(format!("{}", arg)));
                    }
                    return Err("texto() requiere 1 argumento.".into());
                }

                if name == "json_parsear" {
                    if let Some(RuntimeValue::Text(raw)) = eval_args.get(0) {
                        let json = serde_json::from_str::<serde_json::Value>(raw)
                            .map_err(|e| format!("json_parsear(): JSON inválido: {}", e))?;
                        return Ok(crate::servidor::json_to_nexus(&json));
                    }
                    return Err("json_parsear() requiere texto JSON.".into());
                }

                if name == "json_texto" {
                    if let Some(arg) = eval_args.get(0) {
                        let json = runtime_to_json_value(arg)?;
                        let text = serde_json::to_string(&json)
                            .map_err(|e| format!("json_texto(): no se pudo serializar: {}", e))?;
                        return Ok(RuntimeValue::Text(text));
                    }
                    return Err("json_texto() requiere 1 argumento.".into());
                }
                
                if name == "mayusculas" {
                    if let Some(RuntimeValue::Text(s)) = eval_args.get(0) { return Ok(RuntimeValue::Text(s.to_uppercase())); }
                    return Err("mayusculas() requiere texto.".into());
                }
                if name == "minusculas" {
                    if let Some(RuntimeValue::Text(s)) = eval_args.get(0) { return Ok(RuntimeValue::Text(s.to_lowercase())); }
                    return Err("minusculas() requiere texto.".into());
                }
                if name == "contiene" {
                    if let (Some(RuntimeValue::Text(t)), Some(RuntimeValue::Text(s))) = (eval_args.get(0), eval_args.get(1)) { 
                        return Ok(RuntimeValue::Boolean(t.contains(s))); 
                    }
                    return Err("contiene(t, s) requiere dos textos.".into());
                }
                if name == "dividir" {
                    if let (Some(RuntimeValue::Text(t)), Some(RuntimeValue::Text(s))) = (eval_args.get(0), eval_args.get(1)) { 
                        let items: Vec<RuntimeValue> = t.split(s).map(|p| RuntimeValue::Text(p.to_string())).collect();
                        return Ok(RuntimeValue::List(Arc::new(Mutex::new(items))));
                    }
                    return Err("dividir(t, s) requiere dos textos.".into());
                }
                if name == "unir" {
                    if let (Some(RuntimeValue::List(l)), Some(RuntimeValue::Text(s))) = (eval_args.get(0), eval_args.get(1)) { 
                        let items: Vec<String> = l.lock().unwrap().iter().map(|p| format!("{}", p)).collect();
                        return Ok(RuntimeValue::Text(items.join(s)));
                    }
                    return Err("unir(lista, sep) requiere lista y separador texto.".into());
                }

                if name == "todo" {
                    if let Some(RuntimeValue::List(l_arc)) = eval_args.get(0) {
                        let l = l_arc.lock().unwrap();
                        let promise_data = Arc::new((Mutex::new(PromiseState::Pending), Condvar::new()));
                        let promise_data_thread = Arc::clone(&promise_data);
                        let promises: Vec<RuntimeValue> = l.iter().cloned().collect();

                        std::thread::spawn(move || {
                            let mut results = Vec::new();
                            for item in promises {
                                if let RuntimeValue::Promise(p) = item {
                                    let (lock, cvar) = &*p;
                                    let mut state = lock.lock().unwrap();
                                    while let PromiseState::Pending = *state {
                                        state = cvar.wait(state).unwrap();
                                    }
                                    match &*state {
                                        PromiseState::Resolved(v) => results.push((**v).clone()),
                                        PromiseState::Rejected(e) => {
                                            let mut final_state = promise_data_thread.0.lock().unwrap();
                                            *final_state = PromiseState::Rejected(e.clone());
                                            promise_data_thread.1.notify_all();
                                            return;
                                        },
                                        _ => unreachable!(),
                                    }
                                } else {
                                    results.push(item);
                                }
                            }
                            let mut final_state = promise_data_thread.0.lock().unwrap();
                            *final_state = PromiseState::Resolved(Box::new(RuntimeValue::List(Arc::new(Mutex::new(results)))));
                            promise_data_thread.1.notify_all();
                        });

                        return Ok(RuntimeValue::Promise(promise_data));
                    }
                    return Err("todo() requiere una lista de promesas.".into());
                }
                if name == "agregar" {
                    if eval_args.len() == 2 {
                        if let Some(RuntimeValue::List(l)) = eval_args.get(0) {
                            l.lock().unwrap().push(eval_args[1].clone());
                            return Ok(RuntimeValue::Null);
                        }
                    }
                    return Err("agregar(lista, elemento) requiere lista y elemento.".into());
                }
                if name == "quitar" {
                    if let (Some(RuntimeValue::List(l)), Some(RuntimeValue::Int(i))) = (eval_args.get(0), eval_args.get(1)) {
                        let mut l_lock = l.lock().unwrap();
                        if *i >= 0 && (*i as usize) < l_lock.len() {
                            l_lock.remove(*i as usize);
                            return Ok(RuntimeValue::Null);
                        }
                        return Err(format!("quitar(): índice fuera de límites: {}", i));
                    }
                    return Err("quitar(lista, indice_entero) inválido.".into());
                }
                
                if name == "http_get" {
                    if let Some(RuntimeValue::Text(url)) = eval_args.get(0).cloned() {
                        let promise_data = Arc::new((Mutex::new(PromiseState::Pending), Condvar::new()));
                        let promise_data_thread = Arc::clone(&promise_data);
                        std::thread::spawn(move || {
                            let result = ureq::get(&url).call();
                            let mut state = promise_data_thread.0.lock().unwrap();
                            match result {
                                Ok(resp) => {
                                    if let Ok(json) = resp.into_json::<serde_json::Value>() {
                                        let dict = crate::servidor::json_to_nexus(&json);
                                        *state = PromiseState::Resolved(Box::new(dict));
                                    } else {
                                        *state = PromiseState::Rejected("Respuesta de GET no es un JSON válido.".into());
                                    }
                                },
                                Err(e) => {
                                    *state = PromiseState::Rejected(format!("Error HTTP GET a {}: {}", url, e));
                                }
                            }
                            promise_data_thread.1.notify_all();
                        });
                        return Ok(RuntimeValue::Promise(promise_data));
                    }
                    return Err("http_get() requiere una url de texto.".into());
                }

                if name == "http_post" {
                    if let (Some(RuntimeValue::Text(url)), Some(body_dict)) = (eval_args.get(0), eval_args.get(1)) {
                        let url = url.clone();
                        let body_dict = body_dict.clone();
                        let promise_data = Arc::new((Mutex::new(PromiseState::Pending), Condvar::new()));
                        let promise_data_thread = Arc::clone(&promise_data);
                        std::thread::spawn(move || {
                            let json_str = crate::servidor::nexus_to_json_string(&body_dict);
                            let mut state = promise_data_thread.0.lock().unwrap();
                            if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&json_str) {
                                match ureq::post(&url).send_json(&json_val) {
                                    Ok(resp) => {
                                        if let Ok(json) = resp.into_json::<serde_json::Value>() {
                                            let dict = crate::servidor::json_to_nexus(&json);
                                            *state = PromiseState::Resolved(Box::new(dict));
                                        } else {
                                            *state = PromiseState::Resolved(Box::new(RuntimeValue::Text("Respuesta procesada (No JSON)".into())));
                                        }
                                    },
                                    Err(e) => {
                                        *state = PromiseState::Rejected(format!("Error HTTP POST a {}: {}", url, e));
                                    }
                                }
                            } else {
                                *state = PromiseState::Rejected("Cuerpo de POST inválido.".into());
                            }
                            promise_data_thread.1.notify_all();
                        });
                        return Ok(RuntimeValue::Promise(promise_data));
                    }
                    return Err("http_post() requiere url (texto) y un cuerpo (diccionario/lista).".into());
                }

                if name == "ia" {
                    if let Some(arg) = eval_args.get(0) {
                        let prompt = match arg {
                            RuntimeValue::Text(p) => p.clone(),
                            _ => return Err("ia() requiere un texto (prompt).".into())
                        };

                        let mut api_key = std::env::var("NEUROCODE_AI_KEY")
                            .or_else(|_| std::env::var("AQUILA_AI_KEY"))
                            .unwrap_or_default();
                        
                        let ollama_urls = ollama_generate_urls();
                        let mut url = ollama_urls
                            .first()
                            .cloned()
                            .unwrap_or_else(|| "http://localhost:11434/api/generate".to_string());
                        let mut model = "llama3.2:latest".to_string();

                        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_else(|_| ".".to_string());
                        let config_str = std::fs::read_to_string(format!("{}/.neurocode_keys", home))
                            .or_else(|_| std::fs::read_to_string(format!("{}/.aquila_keys", home)));
                        if let Ok(config_str) = config_str {
                            if let Ok(config_json) = serde_json::from_str::<serde_json::Value>(&config_str) {
                                if let Some(u) = config_json.get("url").and_then(|u| u.as_str()) { url = u.to_string(); }
                                if let Some(c) = config_json.get("clave").and_then(|c| c.as_str()) { api_key = c.to_string(); }
                                if let Some(m) = config_json.get("modelo").and_then(|m| m.as_str()) { model = m.to_string(); }
                            }
                        }

                        if eval_args.len() >= 2 {
                            if let RuntimeValue::Dictionary(dict_arc) = &eval_args[1] {
                                let dict = dict_arc.lock().unwrap();
                                if let Some(RuntimeValue::Text(u)) = dict.get("url") { url = u.clone(); }
                                if let Some(RuntimeValue::Text(c)) = dict.get("clave") { api_key = c.clone(); }
                                if let Some(RuntimeValue::Text(m)) = dict.get("modelo") { model = m.clone(); }
                            }
                        }

                        let promise_data = Arc::new((Mutex::new(PromiseState::Pending), Condvar::new()));
                        let promise_data_thread = Arc::clone(&promise_data);

                        std::thread::spawn(move || {
                            let is_ollama_generate = ollama_urls.iter().any(|candidate| candidate == &url);
                            let is_anthropic = url.contains("anthropic.com");

                            let body = if is_ollama_generate {
                                serde_json::json!({ "model": model, "prompt": prompt, "stream": false })
                            } else if is_anthropic {
                                serde_json::json!({ "model": model, "max_tokens": 1024, "messages": [{"role": "user", "content": prompt}] })
                            } else {
                                serde_json::json!({ "model": model, "messages": [{"role": "user", "content": prompt}] })
                            };

                            let execute_request = |req_url: &str, req_body: &serde_json::Value, key: &str, anthropic: bool| -> Result<serde_json::Value, ureq::Error> {
                                let mut request = ureq::post(req_url);
                                if !key.is_empty() {
                                    if anthropic {
                                        request = request.set("x-api-key", key).set("anthropic-version", "2023-06-01");
                                    } else {
                                        request = request.set("Authorization", &format!("Bearer {}", key));
                                    }
                                }
                                let resp = request.send_json(req_body)?;
                                Ok(resp.into_json::<serde_json::Value>().unwrap_or_else(|_| serde_json::json!({})))
                            };

                            let result = execute_request(&url, &body, &api_key, is_anthropic);
                            let mut state = promise_data_thread.0.lock().unwrap();
                            match result {
                                Ok(json) => {
                                    let mut response_text = String::new();
                                    if is_ollama_generate {
                                        if let Some(c) = json.get("response").and_then(|c| c.as_str()) { response_text = c.to_string(); }
                                    } else if is_anthropic {
                                        if let Some(content) = json.get("content").and_then(|c| c.as_array()).and_then(|a| a.get(0)).and_then(|o| o.get("text")).and_then(|t| t.as_str()) {
                                            response_text = content.to_string();
                                        }
                                    } else {
                                        if let Some(c) = json.get("choices").and_then(|c| c.get(0)).and_then(|c| c.get("message")).and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
                                            response_text = c.to_string();
                                        }
                                    }
                                    *state = PromiseState::Resolved(Box::new(RuntimeValue::Text(response_text)));
                                },
                                Err(e) => {
                                    *state = PromiseState::Rejected(format!("Error IA: {}", e));
                                }
                            }
                            promise_data_thread.1.notify_all();
                        });

                        return Ok(RuntimeValue::Promise(promise_data));
                    }
                    return Err("ia() requiere al menos 1 argumento (prompt).".into());
                }

                if name == "ia_generar_codigo" {
                    if let Some(RuntimeValue::Text(descripcion)) = eval_args.get(0) {
                        let prompt = format!(
                            "Eres un generador de código NeuroCode. Genera SOLAMENTE código NeuroCode válido sin explicaciones ni markdown.\nGenera código para: {}", 
                            descripcion
                        );

                        let mut api_key = std::env::var("NEUROCODE_AI_KEY").or_else(|_| std::env::var("AQUILA_AI_KEY")).unwrap_or_default();
                        let ollama_urls = ollama_generate_urls();
                        let mut url = ollama_urls.first().cloned().unwrap_or_else(|| "http://localhost:11434/api/generate".to_string());
                        let mut model = "llama3.2:latest".to_string();

                        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_else(|_| ".".to_string());
                        let config_str = std::fs::read_to_string(format!("{}/.neurocode_keys", home)).or_else(|_| std::fs::read_to_string(format!("{}/.aquila_keys", home)));
                        if let Ok(config_str) = config_str {
                            if let Ok(cj) = serde_json::from_str::<serde_json::Value>(&config_str) {
                                if let Some(u) = cj.get("url").and_then(|u| u.as_str()) { url = u.to_string(); }
                                if let Some(c) = cj.get("clave").and_then(|c| c.as_str()) { api_key = c.to_string(); }
                                if let Some(m) = cj.get("modelo").and_then(|m| m.as_str()) { model = m.to_string(); }
                            }
                        }

                        let promise_data = Arc::new((Mutex::new(PromiseState::Pending), Condvar::new()));
                        let promise_data_thread = Arc::clone(&promise_data);
                        let is_ollama = ollama_urls.iter().any(|c| c == &url);

                        std::thread::spawn(move || {
                            let body = if is_ollama {
                                serde_json::json!({"model": model, "prompt": prompt, "stream": false})
                            } else {
                                serde_json::json!({"model": model, "messages": [{"role": "user", "content": prompt}]})
                            };

                            let mut request = ureq::post(&url).timeout(std::time::Duration::from_secs(60));
                            if !api_key.is_empty() {
                                request = request.set("Authorization", &format!("Bearer {}", api_key));
                            }

                            let mut state = promise_data_thread.0.lock().unwrap();
                            if let Ok(resp) = request.send_json(body) {
                                if let Ok(json) = resp.into_json::<serde_json::Value>() {
                                    let content = if is_ollama {
                                        json.get("response").and_then(|r| r.as_str()).map(|s| s.to_string())
                                    } else {
                                        json.get("choices").and_then(|c| c.get(0)).and_then(|c| c.get("message")).and_then(|m| m.get("content")).and_then(|c| c.as_str()).map(|s| s.to_string())
                                    };
                                    if let Some(code) = content {
                                        let clean = code.trim().trim_start_matches("```neuro").trim_start_matches("```").trim_end_matches("```").trim().to_string();
                                        *state = PromiseState::Resolved(Box::new(RuntimeValue::Text(clean)));
                                    } else {
                                        *state = PromiseState::Resolved(Box::new(RuntimeValue::Text("// No se pudo generar código".to_string())));
                                    }
                                }
                            } else {
                                *state = PromiseState::Rejected("Error de red en ia_generar_codigo".into());
                            }
                            promise_data_thread.1.notify_all();
                        });
                        return Ok(RuntimeValue::Promise(promise_data));
                    }
                    return Err("ia_generar_codigo() requiere una descripción (texto).".into());
                }

                if name == "leer_archivo" {
                    if let Some(RuntimeValue::Text(ruta)) = eval_args.get(0) {
                        match std::fs::read_to_string(ruta) {
                            Ok(c) => return Ok(RuntimeValue::Text(c)),
                            Err(e) => return Err(format!("Error al leer el archivo {}: {}", ruta, e)),
                        }
                    }
                    return Err("leer_archivo() requiere la ruta en texto.".into());
                }

                if name == "archivo_existe" {
                    if let Some(RuntimeValue::Text(ruta)) = eval_args.get(0) {
                        return Ok(RuntimeValue::Boolean(std::path::Path::new(ruta).exists()));
                    }
                    return Err("archivo_existe() requiere la ruta en texto.".into());
                }

                if name == "escribir_archivo" {
                    if eval_args.len() == 2 {
                        if let (RuntimeValue::Text(ruta), RuntimeValue::Text(data)) = (&eval_args[0], &eval_args[1]) {
                            match std::fs::write(ruta, data) {
                                Ok(_) => return Ok(RuntimeValue::Null),
                                Err(e) => return Err(format!("Error al escribir el archivo {}: {}", ruta, e)),
                            }
                        }
                    }
                    return Err("escribir_archivo(ruta, datos) requiere 2 argumentos de texto.".into());
                }

                if name == "timestamp" {
                    let seconds = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    return Ok(RuntimeValue::Int(seconds));
                }

                if name == "dormir" {
                    if let Some(arg) = eval_args.get(0) {
                        let millis = match arg {
                            RuntimeValue::Int(ms) => *ms,
                            RuntimeValue::Number(ms) => *ms as i64,
                            _ => return Err("dormir() requiere milisegundos como número.".into()),
                        };
                        if millis < 0 {
                            return Err("dormir() no acepta tiempos negativos.".into());
                        }
                        std::thread::sleep(std::time::Duration::from_millis(millis as u64));
                        return Ok(RuntimeValue::Null);
                    }
                    return Err("dormir() requiere milisegundos.".into());
                }
                let func_val_res = env.lock().unwrap().get(&name);
                if let Ok(func_val) = func_val_res {
                    match func_val {
                        RuntimeValue::Function(params, return_type, body) => {
                            if args.len() != params.len() {
                                return Err(format!("La función '{}' esperaba {} argumentos pero recibió {}.", name, params.len(), args.len()));
                            }
                            let call_env = Arc::new(Mutex::new(Environment::new_with_parent(Arc::clone(env))));
                            bind_params(&params, &eval_args, &call_env)?;
                            if let Some(ret) = self.execute_block(body, &call_env)? {
                                if let Some(type_name) = &return_type {
                                    validate_runtime_type(&ret, type_name)?;
                                }
                                return Ok(ret);
                            }
                            return Ok(RuntimeValue::Null);
                        },
                        RuntimeValue::AsyncFunction(params, return_type, body) => {
                            if args.len() != params.len() {
                                return Err(format!("La función asíncrona '{}' esperaba {} argumentos pero recibió {}.", name, params.len(), args.len()));
                            }
                            let call_env = Arc::new(Mutex::new(Environment::new_with_parent(Arc::clone(env))));
                            bind_params(&params, &eval_args, &call_env)?;

                            let promise_data = Arc::new((Mutex::new(PromiseState::Pending), Condvar::new()));
                            let promise_data_thread = Arc::clone(&promise_data);
                            
                            let mut interpreter_thread = self.clone();
                            let body_thread = body.clone();
                            let return_type_thread = return_type.clone();

                            std::thread::spawn(move || {
                                match interpreter_thread.execute_block(body_thread, &call_env) {
                                    Ok(Some(ret)) => {
                                        if let Some(type_name) = &return_type_thread {
                                            if let Err(e) = validate_runtime_type(&ret, type_name) {
                                                let mut state = promise_data_thread.0.lock().unwrap();
                                                *state = PromiseState::Rejected(e);
                                                promise_data_thread.1.notify_all();
                                                return;
                                            }
                                        }
                                        let mut state = promise_data_thread.0.lock().unwrap();
                                        *state = PromiseState::Resolved(Box::new(ret));
                                        promise_data_thread.1.notify_all();
                                    },
                                    Ok(None) => {
                                        let mut state = promise_data_thread.0.lock().unwrap();
                                        *state = PromiseState::Resolved(Box::new(RuntimeValue::Null));
                                        promise_data_thread.1.notify_all();
                                    },
                                    Err(e) => {
                                        let mut state = promise_data_thread.0.lock().unwrap();
                                        *state = PromiseState::Rejected(e);
                                        promise_data_thread.1.notify_all();
                                    }
                                }
                            });

                            return Ok(RuntimeValue::Promise(promise_data));
                        },
                        _ => {}
                    }
                } else {
                    return Err(format!("Llamada a función desconocida: {}", name));
                }
                Ok(RuntimeValue::Null) // Se agregó para consistencia del match
            },
            Expression::MethodCall(callee_expr, method, args) => {
                let callee_val = self.evaluate(*callee_expr, env)?;
                let mut eval_args = Vec::new();
                for arg in args {
                    eval_args.push(self.evaluate(arg, env)?);
                }
                
                // Servidor web: interceptar .ruta() e .iniciar()
                if let RuntimeValue::Server(server_arc) = &callee_val {
                    if method == "ruta" {
                        if eval_args.len() >= 3 {
                            let http_method = match &eval_args[0] {
                                RuntimeValue::Text(m) => m.clone(),
                                _ => return Err("servidor.ruta() primer arg debe ser 'GET' o 'POST'.".into()),
                            };
                            let path = match &eval_args[1] {
                                RuntimeValue::Text(p) => p.clone(),
                                _ => return Err("servidor.ruta() segundo arg debe ser la ruta como texto.".into()),
                            };
                            let handler = match &eval_args[2] {
                                RuntimeValue::Function(p, _, b) => (param_names(p), b.clone()),
                                RuntimeValue::AsyncFunction(p, _, b) => (param_names(p), b.clone()),
                                _ => return Err("servidor.ruta() tercer arg debe ser una función (síncrona o asíncrona).".into()),
                            };
                            server_arc.add_route(http_method, path, handler.0, handler.1);
                            return Ok(RuntimeValue::Null);
                        }
                        return Err("servidor.ruta(metodo, path, handler) requiere 3 argumentos.".into());
                    }
                    if method == "estatico" {
                        if eval_args.len() >= 2 {
                            if let (RuntimeValue::Text(path), RuntimeValue::Text(file_path)) = (&eval_args[0], &eval_args[1]) {
                                server_arc.add_static(path.clone(), file_path.clone());
                                return Ok(RuntimeValue::Null);
                            }
                        }
                        return Err("servidor.estatico(ruta, archivo_local) requiere 2 argumentos de texto.".into());
                    }
                    if method == "iniciar" {
                        let server = Arc::clone(server_arc);
                        return server.start(self, env)
                            .map(|_| RuntimeValue::Null);
                    }
                    return Err(format!("Servidor no tiene método '{}'. Usa .ruta() o .iniciar()", method));
                }
                
                // Base de datos: interceptar .ejecutar() y .consultar()
                if let RuntimeValue::Database(db_arc) = &callee_val {
                    if method == "ejecutar" {
                        if let Some(RuntimeValue::Text(sql)) = eval_args.get(0) {
                            let mut params = Vec::new();
                            if let Some(RuntimeValue::List(l)) = eval_args.get(1) {
                                for p in l.lock().unwrap().iter() {
                                    params.push(format!("{}", p));
                                }
                            }
                            return db_arc.ejecutar(sql, params);
                        }
                        return Err("db.ejecutar() requiere un texto SQL.".into());
                    }
                    if method == "consultar" {
                        if let Some(RuntimeValue::Text(sql)) = eval_args.get(0) {
                            let mut params = Vec::new();
                            if let Some(RuntimeValue::List(l)) = eval_args.get(1) {
                                for p in l.lock().unwrap().iter() {
                                    params.push(format!("{}", p));
                                }
                            }
                            return db_arc.consultar(sql, params);
                        }
                        return Err("db.consultar() requiere un texto SQL.".into());
                    }
                    return Err(format!("BaseDatos no tiene método '{}'. Usa .ejecutar() o .consultar()", method));
                }
                
                // Acceso a propiedad u función de diccionario (ej: modulo.PI o modulo.sumar(a,b))
                if let RuntimeValue::Dictionary(map_arc) = &callee_val {
                    if eval_args.is_empty() {
                        let map = map_arc.lock().unwrap();
                        if let Some(val) = map.get(&method) {
                            if let RuntimeValue::Function(params, return_type, body) = val {
                                if params.is_empty() {
                                    let call_env = Arc::new(Mutex::new(Environment::new_with_parent(Arc::clone(env))));
                                    if let Some(ret) = self.execute_block(body.clone(), &call_env)? {
                                        if let Some(type_name) = return_type {
                                            validate_runtime_type(&ret, type_name)?;
                                        }
                                        return Ok(ret);
                                    }
                                    return Ok(RuntimeValue::Null);
                                }
                            }
                            if let RuntimeValue::AsyncFunction(params, return_type, body) = val {
                                if params.is_empty() {
                                    let call_env = Arc::new(Mutex::new(Environment::new_with_parent(Arc::clone(env))));
                                    let promise_data = Arc::new((Mutex::new(PromiseState::Pending), Condvar::new()));
                                    let promise_data_thread = Arc::clone(&promise_data);
                                    let mut interpreter_thread = self.clone();
                                    let body_thread = body.clone();
                                    let return_type_thread = return_type.clone();

                                    std::thread::spawn(move || {
                                        match interpreter_thread.execute_block(body_thread, &call_env) {
                                            Ok(Some(ret)) => {
                                                if let Some(type_name) = &return_type_thread {
                                                    if let Err(e) = validate_runtime_type(&ret, type_name) {
                                                        let mut state = promise_data_thread.0.lock().unwrap();
                                                        *state = PromiseState::Rejected(e);
                                                        promise_data_thread.1.notify_all();
                                                        return;
                                                    }
                                                }
                                                let mut state = promise_data_thread.0.lock().unwrap();
                                                *state = PromiseState::Resolved(Box::new(ret));
                                                promise_data_thread.1.notify_all();
                                            },
                                            Ok(None) => {
                                                let mut state = promise_data_thread.0.lock().unwrap();
                                                *state = PromiseState::Resolved(Box::new(RuntimeValue::Null));
                                                promise_data_thread.1.notify_all();
                                            },
                                            Err(e) => {
                                                let mut state = promise_data_thread.0.lock().unwrap();
                                                *state = PromiseState::Rejected(e);
                                                promise_data_thread.1.notify_all();
                                            }
                                        }
                                    });
                                    return Ok(RuntimeValue::Promise(promise_data));
                                }
                            }
                            return Ok(val.clone());
                        }
                        return Ok(RuntimeValue::Null); // O error si no existe
                    } else {
                        let map = map_arc.lock().unwrap();
                        if let Some(val) = map.get(&method) {
                            if let RuntimeValue::Function(params, return_type, body) = val {
                                if eval_args.len() != params.len() {
                                    return Err(format!("La función '{}' esperaba {} argumentos pero recibió {}.", method, params.len(), eval_args.len()));
                                }
                                let call_env = Arc::new(Mutex::new(Environment::new_with_parent(Arc::clone(env))));
                                bind_params(params, &eval_args, &call_env)?;
                                if let Some(ret) = self.execute_block(body.clone(), &call_env)? {
                                    if let Some(type_name) = return_type {
                                        validate_runtime_type(&ret, type_name)?;
                                    }
                                    return Ok(ret);
                                }
                                return Ok(RuntimeValue::Null);
                            } else if let RuntimeValue::AsyncFunction(params, return_type, body) = val {
                                if eval_args.len() != params.len() {
                                    return Err(format!("La función asíncrona '{}' esperaba {} argumentos pero recibió {}.", method, params.len(), eval_args.len()));
                                }
                                let call_env = Arc::new(Mutex::new(Environment::new_with_parent(Arc::clone(env))));
                                bind_params(params, &eval_args, &call_env)?;

                                let promise_data = Arc::new((Mutex::new(PromiseState::Pending), Condvar::new()));
                                let promise_data_thread = Arc::clone(&promise_data);
                                let mut interpreter_thread = self.clone();
                                let body_thread = body.clone();
                                let return_type_thread = return_type.clone();

                                std::thread::spawn(move || {
                                    match interpreter_thread.execute_block(body_thread, &call_env) {
                                        Ok(Some(ret)) => {
                                            if let Some(type_name) = &return_type_thread {
                                                if let Err(e) = validate_runtime_type(&ret, type_name) {
                                                    let mut state = promise_data_thread.0.lock().unwrap();
                                                    *state = PromiseState::Rejected(e);
                                                    promise_data_thread.1.notify_all();
                                                    return;
                                                }
                                            }
                                            let mut state = promise_data_thread.0.lock().unwrap();
                                            *state = PromiseState::Resolved(Box::new(ret));
                                            promise_data_thread.1.notify_all();
                                        },
                                        Ok(None) => {
                                            let mut state = promise_data_thread.0.lock().unwrap();
                                            *state = PromiseState::Resolved(Box::new(RuntimeValue::Null));
                                            promise_data_thread.1.notify_all();
                                        },
                                        Err(e) => {
                                            let mut state = promise_data_thread.0.lock().unwrap();
                                            *state = PromiseState::Rejected(e);
                                            promise_data_thread.1.notify_all();
                                        }
                                    }
                                });

                                return Ok(RuntimeValue::Promise(promise_data));
                            }
                        }
                    }
                }
                
                // Métodos nativos de Texto
                if let RuntimeValue::Text(s) = &callee_val {
                    if method == "longitud" {
                        return Ok(RuntimeValue::Int(s.len() as i64));
                    }
                    if method == "contiene" {
                        if let Some(RuntimeValue::Text(sub)) = eval_args.get(0) {
                            return Ok(RuntimeValue::Boolean(s.contains(sub)));
                        }
                    }
                }

                // Métodos nativos de Lista
                if let RuntimeValue::List(items_arc) = &callee_val {
                    if method == "mapa" {
                        if let Some(func_val) = eval_args.get(0) {
                            let mut results = Vec::new();
                            let items = items_arc.lock().unwrap().clone();
                            for item in items {
                                if let RuntimeValue::Function(params, return_type, body) = func_val {
                                    let call_env = Arc::new(Mutex::new(Environment::new_with_parent(Arc::clone(env))));
                                    bind_params(params, &vec![item], &call_env)?;
                                    let ret = self.execute_block(body.clone(), &call_env)?.unwrap_or(RuntimeValue::Null);
                                    if let Some(type_name) = return_type {
                                        validate_runtime_type(&ret, type_name)?;
                                    }
                                    results.push(ret);
                                } else {
                                    return Err("El argumento de 'mapa' debe ser una función normal.".into());
                                }
                            }
                            return Ok(RuntimeValue::List(Arc::new(Mutex::new(results))));
                        }
                        return Err("El método 'mapa' requiere una función como argumento.".into());
                    }
                    if method == "filtrar" {
                        if let Some(func_val) = eval_args.get(0) {
                            let mut results = Vec::new();
                            let items = items_arc.lock().unwrap().clone();
                            for item in items {
                                if let RuntimeValue::Function(params, _, body) = func_val {
                                    let call_env = Arc::new(Mutex::new(Environment::new_with_parent(Arc::clone(env))));
                                    bind_params(params, &vec![item.clone()], &call_env)?;
                                    let ret = self.execute_block(body.clone(), &call_env)?.unwrap_or(RuntimeValue::Null);
                                    if self.is_truthy(&ret) {
                                        results.push(item);
                                    }
                                } else {
                                    return Err("El argumento de 'filtrar' debe ser una función normal.".into());
                                }
                            }
                            return Ok(RuntimeValue::List(Arc::new(Mutex::new(results))));
                        }
                        return Err("El método 'filtrar' requiere una función como argumento.".into());
                    }
                    if method == "reducir" {
                        if eval_args.len() < 2 {
                            return Err("El método 'reducir' requiere una función y un valor inicial.".into());
                        }
                        if let (Some(func_val), Some(init_val)) = (eval_args.get(0), eval_args.get(1)) {
                            let mut acc = init_val.clone();
                            let items = items_arc.lock().unwrap().clone();
                            for item in items {
                                if let RuntimeValue::Function(params, return_type, body) = func_val {
                                    let call_env = Arc::new(Mutex::new(Environment::new_with_parent(Arc::clone(env))));
                                    // bind accumulated value AND current item
                                    bind_params(params, &vec![acc.clone(), item], &call_env)?;
                                    acc = self.execute_block(body.clone(), &call_env)?.unwrap_or(RuntimeValue::Null);
                                    if let Some(type_name) = return_type {
                                        validate_runtime_type(&acc, type_name)?;
                                    }
                                } else {
                                    return Err("El primer argumento de 'reducir' debe ser una función normal.".into());
                                }
                            }
                            return Ok(acc);
                        }
                    }
                    if method == "ordenar" {
                        let mut items = items_arc.lock().unwrap().clone();
                        items.sort_by(|a, b| {
                            match (a, b) {
                                (RuntimeValue::Int(x), RuntimeValue::Int(y)) => x.cmp(y),
                                (RuntimeValue::Number(x), RuntimeValue::Number(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                                (RuntimeValue::Text(x), RuntimeValue::Text(y)) => x.cmp(y),
                                (RuntimeValue::Int(x), RuntimeValue::Number(y)) => (*x as f64).partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                                (RuntimeValue::Number(x), RuntimeValue::Int(y)) => x.partial_cmp(&(*y as f64)).unwrap_or(std::cmp::Ordering::Equal),
                                _ => std::cmp::Ordering::Equal,
                            }
                        });
                        *items_arc.lock().unwrap() = items;
                        return Ok(callee_val.clone());
                    }
                    if method == "reverso" {
                        let mut items = items_arc.lock().unwrap().clone();
                        items.reverse();
                        *items_arc.lock().unwrap() = items;
                        return Ok(callee_val.clone());
                    }
                    if method == "buscar" {
                        if let Some(func_val) = eval_args.get(0) {
                            let items = items_arc.lock().unwrap().clone();
                            for item in items {
                                if let RuntimeValue::Function(params, _, body) = func_val {
                                    let call_env = Arc::new(Mutex::new(Environment::new_with_parent(Arc::clone(env))));
                                    bind_params(params, &vec![item.clone()], &call_env)?;
                                    let ret = self.execute_block(body.clone(), &call_env)?.unwrap_or(RuntimeValue::Null);
                                    if self.is_truthy(&ret) {
                                        return Ok(item);
                                    }
                                } else {
                                    return Err("El argumento de 'buscar' debe ser una función normal.".into());
                                }
                            }
                            return Ok(RuntimeValue::Null);
                        }
                        return Err("El método 'buscar' requiere una función como argumento.".into());
                    }
                }

                // Acceso a propiedad u invocación de objeto instanciado
                if let RuntimeValue::Instance(_, props_arc, class_box) = &callee_val {
                    if eval_args.is_empty() {
                        let props = props_arc.lock().unwrap();
                        if let Some(val) = props.get(&method) {
                            return Ok(val.clone());
                        }
                    }
                    
                    if let RuntimeValue::Class(_, _, _) = &**class_box {
                        let mut current_class = Some((**class_box).clone());
                        let mut method_found = None;

                        while let Some(cls) = current_class {
                            if let RuntimeValue::Class(_, ref s_name, ref m_arc) = cls {
                                let m = m_arc.lock().unwrap();
                                if let Some(f) = m.get(&method) {
                                    method_found = Some(f.clone());
                                    break;
                                }
                                if let Some(sn) = s_name {
                                    current_class = env.lock().unwrap().get(sn).ok();
                                } else {
                                    current_class = None;
                                }
                            } else {
                                current_class = None;
                            }
                        }

                        if let Some(val) = method_found {
                            if let RuntimeValue::Function(params, return_type, body) = val {
                                if eval_args.len() != params.len() {
                                    return Err(format!("El método '{}' esperaba {} argumentos pero recibió {}.", method, params.len(), eval_args.len()));
                                }
                                let call_env = Arc::new(Mutex::new(Environment::new_with_parent(Arc::clone(env))));
                                call_env.lock().unwrap().define("esto".to_string(), callee_val.clone());
                                bind_params(&params, &eval_args, &call_env)?;
                                if let Some(ret) = self.execute_block(body.clone(), &call_env)? {
                                    if let Some(type_name) = return_type {
                                        validate_runtime_type(&ret, &type_name)?;
                                    }
                                    return Ok(ret);
                                }
                                return Ok(RuntimeValue::Null);
                            } else if let RuntimeValue::AsyncFunction(params, return_type, body) = val {
                                let mut call_env_inner = Environment::new_with_parent(Arc::clone(env));
                                call_env_inner.define("esto".to_string(), callee_val.clone());
                                let call_env = Arc::new(Mutex::new(call_env_inner));
                                bind_params(&params, &eval_args, &call_env)?;
                                
                                let promise_data = Arc::new((Mutex::new(PromiseState::Pending), Condvar::new()));
                                let promise_data_thread = Arc::clone(&promise_data);
                                let mut interpreter_thread = self.clone();
                                let body_thread = body.clone();
                                let return_type_thread = return_type.clone();

                                std::thread::spawn(move || {
                                    match interpreter_thread.execute_block(body_thread, &call_env) {
                                        Ok(Some(ret)) => {
                                            if let Some(tn) = &return_type_thread {
                                                if let Err(e) = validate_runtime_type(&ret, tn) {
                                                    let mut state = promise_data_thread.0.lock().unwrap();
                                                    *state = PromiseState::Rejected(e);
                                                    promise_data_thread.1.notify_all();
                                                    return;
                                                }
                                            }
                                            let mut state = promise_data_thread.0.lock().unwrap();
                                            *state = PromiseState::Resolved(Box::new(ret));
                                            promise_data_thread.1.notify_all();
                                        },
                                        Ok(None) => {
                                            let mut state = promise_data_thread.0.lock().unwrap();
                                            *state = PromiseState::Resolved(Box::new(RuntimeValue::Null));
                                            promise_data_thread.1.notify_all();
                                        },
                                        Err(e) => {
                                            let mut state = promise_data_thread.0.lock().unwrap();
                                            *state = PromiseState::Rejected(e);
                                            promise_data_thread.1.notify_all();
                                        }
                                    }
                                });
                                return Ok(RuntimeValue::Promise(promise_data));
                            }
                        }
                    }
                    return Err(format!("La propiedad o método '{}' no existe en esta instancia.", method));
                }
                
                // Si es un Objeto Python envuelto en Nexus
                if let RuntimeValue::PyWrapper(py_obj) = callee_val {
                    return pyo3::Python::with_gil(|py| -> Result<RuntimeValue, String> {
                        if method == "propiedad" {
                            if let Some(RuntimeValue::Text(prop_name)) = eval_args.get(0) {
                                match py_obj.getattr(py, prop_name.as_str()) {
                                    Ok(res) => return Ok(py_to_val(py, res)),
                                    Err(e) => {
                                        e.print(py);
                                        return Err(format!("No existe la propiedad Python '{}'", prop_name));
                                    }
                                }
                            }
                        }

                        let py_args: Vec<PyObject> = eval_args.into_iter().map(|a| val_to_py(py, a)).collect();
                        let py_args_tuple = pyo3::types::PyTuple::new(py, py_args).unwrap();
                        
                        match py_obj.call_method(py, method.as_str(), py_args_tuple, None) {
                            Ok(res) => Ok(py_to_val(py, res)),
                            Err(e) => {
                                e.print(py);
                                Err(format!("Excepción nativa Python al llamar al método '{}'", method))
                            }
                        }
                    });
                }
                
                Err(format!("El objeto no tiene propiedades o métodos invocables. (método intentado: {})", method))
            },
            Expression::IndexAccess(array_expr, index_expr) => {
                let array = self.evaluate(*array_expr, env)?;
                let index = self.evaluate(*index_expr, env)?;
                
                if let RuntimeValue::List(items_arc) = &array {
                    if let RuntimeValue::Int(idx) = index {
                        let items = items_arc.lock().unwrap();
                        if idx >= 0 && (idx as usize) < items.len() {
                            return Ok(items[idx as usize].clone());
                        } else {
                            return Err(format!("Índice fuera de límites: {}", idx));
                        }
                    }
                    Err("El índice de la lista debe ser un número entero.".into())
                } else if let RuntimeValue::Dictionary(map_arc) = &array {
                    let str_key = match index {
                        RuntimeValue::Text(s) => s,
                        _ => return Err("El índice de un diccionario debe ser texto.".into()),
                    };
                    let map = map_arc.lock().unwrap();
                    if let Some(val) = map.get(&str_key) {
                        Ok(val.clone())
                    } else {
                        Ok(RuntimeValue::Null)
                    }
                } else if let RuntimeValue::PyWrapper(py_obj) = &array {
                    return pyo3::Python::with_gil(|py| -> Result<RuntimeValue, String> {
                        let py_idx = val_to_py(py, index);
                        match py_obj.bind(py).get_item(py_idx) {
                            Ok(res) => Ok(py_to_val(py, res.to_object(py))),
                            Err(e) => {
                                e.print(py);
                                Err("Error en acceso por índice en objeto de Python.".into())
                            }
                        }
                    });
                } else {
                    Err("Solo se permite acceso por índice a listas, diccionarios u objetos de Python.".into())
                }
            },
            Expression::SuperCall(method, args) => {
                let esto = env.lock().unwrap().get("esto")?;
                if let RuntimeValue::Instance(_, _, class_box) = &esto {
                    if let RuntimeValue::Class(_, super_name_opt, _) = &**class_box {
                        if let Some(super_name) = super_name_opt {
                            let super_class = env.lock().unwrap().get(super_name)?;
                            let mut method_found = None;
                            let mut current_cls = Some(super_class);
                            while let Some(cls) = current_cls {
                                if let RuntimeValue::Class(_, ref s_name, ref m_arc) = cls {
                                    let m = m_arc.lock().unwrap();
                                    if let Some(f) = m.get(&method) {
                                        method_found = Some(f.clone());
                                        break;
                                    }
                                    current_cls = s_name.as_ref().and_then(|sn| env.lock().unwrap().get(sn).ok());
                                } else { break; }
                            }

                            if let Some(val) = method_found {
                                let mut eval_args = Vec::new();
                                for arg in args {
                                    eval_args.push(self.evaluate(arg, env)?);
                                }
                                if let RuntimeValue::Function(params, return_type, body) = val {
                                    let call_env = Arc::new(Mutex::new(Environment::new_with_parent(Arc::clone(env))));
                                    call_env.lock().unwrap().define("esto".to_string(), esto.clone());
                                    bind_params(&params, &eval_args, &call_env)?;
                                    if let Some(ret) = self.execute_block(body, &call_env)? {
                                        if let Some(tn) = return_type { validate_runtime_type(&ret, &tn)?; }
                                        return Ok(ret);
                                    }
                                    return Ok(RuntimeValue::Null);
                                }
                            }
                        }
                    }
                }
                Err("No se puede usar 'super' en este contexto o método no encontrado en el padre.".into())
            },
            Expression::SuperConstructor(args) => {
                let esto = env.lock().unwrap().get("esto")?;
                if let RuntimeValue::Instance(_, _, class_box) = &esto {
                    if let RuntimeValue::Class(_, Some(super_name), _) = &**class_box {
                        let super_class = env.lock().unwrap().get(super_name)?;
                        if let RuntimeValue::Class(_, _, methods_arc) = super_class {
                            let methods = methods_arc.lock().unwrap();
                            if let Some(RuntimeValue::Function(params, _, body)) = methods.get("crear") {
                                let mut eval_args = Vec::new();
                                for arg in args {
                                    eval_args.push(self.evaluate(arg, env)?);
                                }
                                let call_env = Arc::new(Mutex::new(Environment::new_with_parent(Arc::clone(env))));
                                call_env.lock().unwrap().define("esto".to_string(), esto.clone());
                                bind_params(&params, &eval_args, &call_env)?;
                                self.execute_block(body.clone(), &call_env)?;
                                return Ok(RuntimeValue::Null);
                            }
                        }
                    }
                }
                Err("No se puede usar 'super()' sin superclase o constructor padre.".into())
            }
        }
    }

    fn evaluate_binary(&self, left: RuntimeValue, op: &str, right: RuntimeValue) -> Result<RuntimeValue, String> {
        if let (RuntimeValue::Int(a), RuntimeValue::Int(b)) = (&left, &right) {
            return match op {
                "+" => Ok(RuntimeValue::Int(a + b)),
                "-" => Ok(RuntimeValue::Int(a - b)),
                "*" => Ok(RuntimeValue::Int(a * b)),
                "/" => {
                    if *b == 0 { return Err("División por cero.".into()); }
                    Ok(RuntimeValue::Number(*a as f64 / *b as f64))
                },
                "%" => {
                    if *b == 0 { return Err("Módulo por cero.".into()); }
                    Ok(RuntimeValue::Int(a % b))
                },
                "==" => Ok(RuntimeValue::Boolean(a == b)),
                "!=" => Ok(RuntimeValue::Boolean(a != b)),
                ">" => Ok(RuntimeValue::Boolean(a > b)),
                "<" => Ok(RuntimeValue::Boolean(a < b)),
                ">=" => Ok(RuntimeValue::Boolean(a >= b)),
                "<=" => Ok(RuntimeValue::Boolean(a <= b)),
                _ => Err(format!("Operador numérico desconocido: '{}'", op)),
            };
        }

        let (lx, ly) = match (left.clone(), right.clone()) {
            (RuntimeValue::Int(a), RuntimeValue::Int(b)) => (a as f64, b as f64),
            (RuntimeValue::Number(a), RuntimeValue::Number(b)) => (a, b),
            (RuntimeValue::Int(a), RuntimeValue::Number(b)) => (a as f64, b),
            (RuntimeValue::Number(a), RuntimeValue::Int(b)) => (a, b as f64),
            // Strings
            (RuntimeValue::Text(t1), RuntimeValue::Text(t2)) if op == "+" => {
                return Ok(RuntimeValue::Text(format!("{}{}", t1, t2)));
            },
            (RuntimeValue::Text(_), _) if op == "+" => {
                return Err("No se puede sumar Texto con números u otros tipos directamente.".into());
            },
            (_, RuntimeValue::Text(_)) if op == "+" => {
                return Err("No se puede sumar tipos con Texto directamente.".into());
            },
            _ => return match op {
                "==" => Ok(RuntimeValue::Boolean(left == right)),
                "!=" => Ok(RuntimeValue::Boolean(left != right)),
                _ => Err(format!("No se puede aplicar operador '{}' a '{}' y '{}'", op, left, right))
            }
        };

        match op {
            "+" => Ok(RuntimeValue::Number(lx + ly)),
            "-" => Ok(RuntimeValue::Number(lx - ly)),
            "*" => Ok(RuntimeValue::Number(lx * ly)),
            "/" => {
                if ly == 0.0 { return Err("División por cero.".into()); }
                Ok(RuntimeValue::Number(lx / ly))
            },
            "%" => {
                if ly == 0.0 { return Err("Módulo por cero.".into()); }
                Ok(RuntimeValue::Number(lx % ly))
            },
            "==" => Ok(RuntimeValue::Boolean(lx == ly)),
            "!=" => Ok(RuntimeValue::Boolean(lx != ly)),
            ">" => Ok(RuntimeValue::Boolean(lx > ly)),
            "<" => Ok(RuntimeValue::Boolean(lx < ly)),
            ">=" => Ok(RuntimeValue::Boolean(lx >= ly)),
            "<=" => Ok(RuntimeValue::Boolean(lx <= ly)),
            _ => Err(format!("Operador numérico desconocido: '{}'", op)),
        }
    }

    fn evaluate_unary(&self, op: &str, right: RuntimeValue) -> Result<RuntimeValue, String> {
        match op {
            "-" => {
                match right {
                    RuntimeValue::Int(i) => Ok(RuntimeValue::Int(-i)),
                    RuntimeValue::Number(n) => Ok(RuntimeValue::Number(-n)),
                    _ => Err("El operador '-' solo se aplica a números.".into())
                }
            },
            "no" => {
                Ok(RuntimeValue::Boolean(!self.is_truthy(&right)))
            },
            _ => Err(format!("Operador unario desconocido: '{}'", op)),
        }
    }

    fn is_truthy(&self, val: &RuntimeValue) -> bool {
        match val {
            RuntimeValue::Null => false,
            RuntimeValue::Boolean(b) => *b,
            RuntimeValue::Int(i) => *i != 0,
            RuntimeValue::Number(n) => *n != 0.0,
            RuntimeValue::Text(s) => !s.is_empty(),
            RuntimeValue::List(l) => !l.lock().unwrap().is_empty(),
            RuntimeValue::Dictionary(d) => !d.lock().unwrap().is_empty(),
            RuntimeValue::Function(_, _, _) => true,
            RuntimeValue::AsyncFunction(_, _, _) => true,
            RuntimeValue::Promise(_) => true,
            RuntimeValue::Class(_, _, _) => true,
            RuntimeValue::Instance(_, _, _) => true,
            RuntimeValue::Server(_) => true,
            RuntimeValue::Database(_) => true,
            RuntimeValue::PyWrapper(_) => true,
            RuntimeValue::Break => false,
            RuntimeValue::Continue => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depredactor_hint_lee_aquila_json() {
        let dir = std::env::temp_dir().join(format!(
            "aquila_interpreter_manifest_{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let manifest = serde_json::json!({
            "nombre": "hint",
            "version": "0.1.0",
            "dependencias": {
                "selenium": {
                    "origen": "python",
                    "paquete": "selenium",
                    "version": "*"
                }
            }
        });
        std::fs::write(dir.join("neurocode.json"), serde_json::to_string(&manifest).unwrap()).unwrap();

        let interpreter = Interpreter::with_base_dir(dir.clone());
        let hint = interpreter.python_dependency_hint("selenium.webdriver");

        assert!(hint.contains("está registrada en neurocode.json"), "hint inesperado: {}", hint);
        assert!(hint.contains("neuro instalar python:selenium"), "hint inesperado: {}", hint);

        let _ = std::fs::remove_file(dir.join("neurocode.json"));
        let _ = std::fs::remove_dir(dir);
    }

    #[test]
    fn test_eval_arithmetic() {
        assert_eq!(run_code_expr("1 + 2 * 3"), RuntimeValue::Int(7));
        assert_eq!(run_code_expr("(1 + 2) * 3"), RuntimeValue::Int(9));
        assert_eq!(run_code_expr("10 / 2"), RuntimeValue::Number(5.0));
    }

    #[test]
    fn test_eval_logic() {
        assert_eq!(run_code_expr("verdadero y falso"), RuntimeValue::Boolean(false));
        assert_eq!(run_code_expr("verdadero o falso"), RuntimeValue::Boolean(true));
        assert_eq!(run_code_expr("no verdadero"), RuntimeValue::Boolean(false));
    }

    #[test]
    fn test_eval_if() {
        let code = "
            x = 0
            si verdadero { x = 1 } sino { x = 2 }
        ";
        assert_eq!(get_var(code, "x"), RuntimeValue::Int(1));
        
        let code2 = "
            x = 0
            si falso { x = 1 } sino { x = 2 }
        ";
        assert_eq!(get_var(code2, "x"), RuntimeValue::Int(2));
    }

    #[test]
    fn test_eval_while() {
        let code = "
            x = 0
            mientras x < 5 {
                x = x + 1
            }
        ";
        assert_eq!(get_var(code, "x"), RuntimeValue::Int(5));
    }

    #[test]
    fn test_eval_function_return() {
        let code = "
            funcion duplicar(n) {
                retornar n * 2
            }
            res = duplicar(10)
        ";
        assert_eq!(get_var(code, "res"), RuntimeValue::Int(20));
    }

    #[test]
    fn test_eval_recursion_fibonacci() {
        let code = "
            funcion fib(n) {
                si n <= 1 { retornar n }
                retornar fib(n - 1) + fib(n - 2)
            }
            res = fib(10)
        ";
        assert_eq!(get_var(code, "res"), RuntimeValue::Int(55));
    }

    #[test]
    fn test_eval_scoping() {
        let code = "
            x = 10
            funcion cambiar() {
                x = 20
            }
            cambiar()
            res = x
        ";
        assert_eq!(get_var(code, "res"), RuntimeValue::Int(20));

        let code2 = "
            x = 10
            funcion local() {
                // En NeuroCode actual, asignar a una variable que existe en el padre
                // la sobreescribe si no usamos una palabra clave de declaración local.
                // Verificamos este comportamiento.
                x = 50
            }
            local()
            res = x
        ";
        assert_eq!(get_var(code2, "res"), RuntimeValue::Int(50));
    }

    #[test]
    fn test_error_div_zero() {
        let code = "1 / 0";
        let tokens = crate::lexer::tokenize(code);
        let statements = crate::parser::parse(tokens).unwrap();
        let mut interpreter = Interpreter::new();
        let err = interpreter.interpret(statements).unwrap_err();
        assert!(err.contains("División por cero."));
    }

    #[test]
    fn test_error_undefined_var() {
        let code = "a + 1";
        let tokens = crate::lexer::tokenize(code);
        let statements = crate::parser::parse(tokens).unwrap();
        let mut interpreter = Interpreter::new();
        let err = interpreter.interpret(statements).unwrap_err();
        assert!(err.contains("Variable no definida: 'a'"));
    }

    #[test]
    fn test_error_func_args() {
        let code = "
            funcion f(x) { retornar x }
            f(1, 2)
        ";
        let tokens = crate::lexer::tokenize(code);
        let statements = crate::parser::parse(tokens).unwrap();
        let mut interpreter = Interpreter::new();
        let err = interpreter.interpret(statements).unwrap_err();
        assert!(err.contains("esperaba 1 argumentos pero recibió 2"));
    }

    #[test]
    fn test_error_recovery() {
        let mut interpreter = Interpreter::new();
        
        // Ejecución con error
        let code1 = "x = 1 / 0";
        let tokens1 = crate::lexer::tokenize(code1);
        let stmts1 = crate::parser::parse(tokens1).unwrap();
        let _ = interpreter.interpret(stmts1).unwrap_err();
        
        // El intérprete debe seguir vivo para la siguiente ejecución
        let code2 = "x = 10";
        let tokens2 = crate::lexer::tokenize(code2);
        let stmts2 = crate::parser::parse(tokens2).unwrap();
        interpreter.interpret(stmts2).unwrap();
        
        let res = interpreter.global_env.lock().unwrap().get("x").unwrap();
        assert_eq!(res, RuntimeValue::Int(10));
    }

    #[test]
    fn test_list_methods() {
        let code = "
            numeros = [1, 2, 3, 4]
            mapa_res = numeros.mapa(funcion(n) { retornar n * 2 })
            filtrar_res = numeros.filtrar(funcion(n) { retornar n > 2 })
            reducir_res = numeros.reducir(funcion(a, b) { retornar a + b }, 0)
            
            numeros.reverso()
            reverso_res = numeros[0]
            
            numeros.ordenar()
            orden_res = numeros[0]
            
            buscar_res = numeros.buscar(funcion(n) { retornar n == 3 })
        ";
        let tokens = crate::lexer::tokenize(code);
        let statements = crate::parser::parse(tokens).unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.interpret(statements).unwrap();
        
        let env = interpreter.global_env.lock().unwrap();
        // Verificar map
        if let RuntimeValue::List(l) = env.get("mapa_res").unwrap() {
            let items = l.lock().unwrap();
            assert_eq!(items[0], RuntimeValue::Int(2));
            assert_eq!(items[3], RuntimeValue::Int(8));
        } else { panic!("mapa_res is not a list") }
        
        // Verificar filter
        if let RuntimeValue::List(l) = env.get("filtrar_res").unwrap() {
            let items = l.lock().unwrap();
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], RuntimeValue::Int(3));
        } else { panic!("filtrar_res is not a list") }
        
        // Verificar reduce
        assert_eq!(env.get("reducir_res").unwrap(), RuntimeValue::Int(10));
        
        // Verificar reverso (era 1,2,3,4, revertido es 4,3,2,1)
        assert_eq!(env.get("reverso_res").unwrap(), RuntimeValue::Int(4));
        
        // Verificar ordenar (ordenado debe volver a 1,2,3,4)
        assert_eq!(env.get("orden_res").unwrap(), RuntimeValue::Int(1));
        
        // Verificar buscar
        assert_eq!(env.get("buscar_res").unwrap(), RuntimeValue::Int(3));
    }

    fn run_code_expr(expr: &str) -> RuntimeValue {
        let tokens = crate::lexer::tokenize(expr);
        let statements = crate::parser::parse(tokens).unwrap();
        let mut interpreter = Interpreter::new();
        // Para expresiones sueltas, necesitamos evaluarla directamente
        if let Statement::Expression(e) = &statements[0] {
            interpreter.evaluate(e.clone(), &interpreter.global_env.clone()).unwrap()
        } else {
            panic!("Not an expression");
        }
    }

    fn get_var(code: &str, var_name: &str) -> RuntimeValue {
        let tokens = crate::lexer::tokenize(code);
        let statements = crate::parser::parse(tokens).unwrap();
        let mut interpreter = Interpreter::new();
        interpreter.interpret(statements).unwrap();
        interpreter.global_env.lock().unwrap().get(var_name).unwrap()
    }
}

fn setup_globals(env: &Arc<Mutex<Environment>>) {
    let mut constructor_map = HashMap::new();
    
    // constructor.crear_login()
    constructor_map.insert("crear_login".to_string(), RuntimeValue::Function(vec![], None, vec![
        Statement::Expression(Expression::FunctionCall("imprimir".to_string(), vec![Expression::Text("🏗️ Creando login...".to_string())]))
    ]));

    let constructor_val = RuntimeValue::Dictionary(Arc::new(Mutex::new(constructor_map)));
    env.lock().unwrap().define("constructor".to_string(), constructor_val);
}
