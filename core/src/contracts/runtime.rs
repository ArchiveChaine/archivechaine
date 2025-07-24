//! Runtime d'exécution WASM pour les smart contracts ArchiveChain

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use wasmer::{
    imports, wat2wasm, Cranelift, Engine, Function, FunctionType, Instance, 
    Memory, Module, Store, Type, Value, WasmPtr
};
use crate::crypto::Hash;
use crate::contracts::{ContractError, ContractResult, ContractContext, GasManager};

/// Configuration du runtime WASM
#[derive(Debug, Clone)]
pub struct WasmRuntimeConfig {
    /// Limite de mémoire en pages (64KB par page)
    pub memory_limit_pages: u32,
    /// Timeout d'exécution en millisecondes
    pub execution_timeout_ms: u64,
    /// Activation du debugging
    pub debug_mode: bool,
}

impl Default for WasmRuntimeConfig {
    fn default() -> Self {
        Self {
            memory_limit_pages: 256, // 16MB max
            execution_timeout_ms: 5000, // 5 secondes
            debug_mode: false,
        }
    }
}

/// Résultat d'une exécution de contrat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Données de retour du contrat
    pub return_data: Vec<u8>,
    /// Gas consommé
    pub gas_used: u64,
    /// Logs émis par le contrat
    pub logs: Vec<String>,
    /// Events émis
    pub events: Vec<ContractEvent>,
    /// Changements d'état
    pub state_changes: Vec<StateChange>,
    /// Succès de l'exécution
    pub success: bool,
    /// Message d'erreur en cas d'échec
    pub error_message: Option<String>,
}

/// Event émis par un contrat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractEvent {
    pub name: String,
    pub data: Vec<u8>,
    pub topics: Vec<Hash>,
}

/// Changement d'état effectué par un contrat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChange {
    pub key: Vec<u8>,
    pub old_value: Option<Vec<u8>>,
    pub new_value: Option<Vec<u8>>,
}

/// Exécution d'un contrat avec contexte
#[derive(Debug)]
pub struct ContractExecution {
    /// Instance WASM du contrat
    instance: Instance,
    /// Store Wasmer
    store: Store,
    /// Gestionnaire de gas
    gas_manager: Arc<Mutex<GasManager>>,
    /// Contexte d'exécution
    context: Arc<Mutex<ContractContext>>,
}

/// Runtime principal pour l'exécution des contrats WASM
#[derive(Debug)]
pub struct WasmRuntime {
    /// Engine Wasmer
    engine: Engine,
    /// Configuration du runtime
    config: WasmRuntimeConfig,
    /// Modules compilés (cache)
    compiled_modules: HashMap<Hash, Module>,
}

impl WasmRuntime {
    /// Crée un nouveau runtime WASM
    pub fn new(config: WasmRuntimeConfig) -> ContractResult<Self> {
        let engine = Engine::new(Cranelift::default());
        
        Ok(Self {
            engine,
            config,
            compiled_modules: HashMap::new(),
        })
    }

    /// Compile un module WASM à partir du bytecode
    pub fn compile_module(&mut self, bytecode: &[u8], contract_hash: Hash) -> ContractResult<()> {
        let module = Module::new(&self.engine, bytecode)
            .map_err(|e| ContractError::WasmExecution { 
                message: format!("Compilation failed: {}", e) 
            })?;
        
        self.compiled_modules.insert(contract_hash, module);
        Ok(())
    }

    /// Charge un contrat et prépare son exécution
    pub fn load_contract(
        &self,
        contract_hash: Hash,
        context: ContractContext,
        gas_limit: u64,
    ) -> ContractResult<ContractExecution> {
        let module = self.compiled_modules.get(&contract_hash)
            .ok_or(ContractError::ContractNotFound { address: contract_hash })?;

        let mut store = Store::new(&self.engine);
        let gas_manager = Arc::new(Mutex::new(GasManager::new(gas_limit)));
        let context_arc = Arc::new(Mutex::new(context));

        // Crée les imports pour les fonctions host
        let imports = self.create_imports(&mut store, gas_manager.clone(), context_arc.clone())?;

        // Instancie le module
        let instance = Instance::new(&mut store, module, &imports)
            .map_err(|e| ContractError::WasmExecution { 
                message: format!("Instantiation failed: {}", e) 
            })?;

        Ok(ContractExecution {
            instance,
            store,
            gas_manager,
            context: context_arc,
        })
    }

    /// Crée les imports (fonctions host) pour les contrats
    fn create_imports(
        &self,
        store: &mut Store,
        gas_manager: Arc<Mutex<GasManager>>,
        context: Arc<Mutex<ContractContext>>,
    ) -> ContractResult<wasmer::Imports> {
        // Fonction pour lire le storage
        let gas_manager_read = gas_manager.clone();
        let context_read = context.clone();
        let storage_read = Function::new_typed(store, move |ptr: u32, len: u32| -> u32 {
            let mut gas = gas_manager_read.lock().unwrap();
            if gas.consume(100).is_err() {
                return 0; // Erreur de gas
            }

            let ctx = context_read.lock().unwrap();
            // Implementation de lecture du storage
            // Pour l'instant, retourne 0 (pas de données)
            0
        });

        // Fonction pour écrire dans le storage
        let gas_manager_write = gas_manager.clone();
        let context_write = context.clone();
        let storage_write = Function::new_typed(store, move |key_ptr: u32, key_len: u32, value_ptr: u32, value_len: u32| -> u32 {
            let mut gas = gas_manager_write.lock().unwrap();
            if gas.consume(200).is_err() {
                return 0; // Erreur de gas
            }

            let mut ctx = context_write.lock().unwrap();
            // Implementation d'écriture du storage
            1 // Succès
        });

        // Fonction pour émettre un log
        let gas_manager_log = gas_manager.clone();
        let log_fn = Function::new_typed(store, move |ptr: u32, len: u32| {
            let mut gas = gas_manager_log.lock().unwrap();
            let _ = gas.consume(50);
            // Implementation du logging
        });

        // Fonction pour émettre un event
        let gas_manager_event = gas_manager.clone();
        let event_fn = Function::new_typed(store, move |name_ptr: u32, name_len: u32, data_ptr: u32, data_len: u32| {
            let mut gas = gas_manager_event.lock().unwrap();
            let _ = gas.consume(300);
            // Implementation des events
        });

        // Fonction pour obtenir le timestamp actuel
        let timestamp_fn = Function::new_typed(store, || -> u64 {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        });

        // Fonction pour obtenir l'adresse du contrat
        let contract_address_fn = Function::new_typed(store, move |ptr: u32| {
            // Écrit l'adresse du contrat à l'adresse donnée
        });

        let imports = imports! {
            "env" => {
                "storage_read" => storage_read,
                "storage_write" => storage_write,
                "log" => log_fn,
                "emit_event" => event_fn,
                "get_timestamp" => timestamp_fn,
                "get_contract_address" => contract_address_fn,
            }
        };

        Ok(imports)
    }
}

impl ContractExecution {
    /// Exécute une fonction du contrat
    pub fn call_function(
        &mut self,
        function_name: &str,
        args: &[Value],
    ) -> ContractResult<ExecutionResult> {
        // Vérifie que la fonction existe
        let function = self.instance
            .exports
            .get_function(function_name)
            .map_err(|_| ContractError::FunctionNotFound { 
                function: function_name.to_string() 
            })?;

        // Exécute la fonction
        let start_gas = {
            let gas = self.gas_manager.lock().unwrap();
            gas.remaining()
        };

        let result = function.call(&mut self.store, args)
            .map_err(|e| ContractError::WasmExecution { 
                message: format!("Function execution failed: {}", e) 
            })?;

        let end_gas = {
            let gas = self.gas_manager.lock().unwrap();
            gas.remaining()
        };

        let gas_used = start_gas - end_gas;

        // Récupère les données de retour
        let return_data = if let Some(Value::I32(ptr)) = result.get(0) {
            self.read_memory(*ptr as u32, 0)? // Simplifié pour l'exemple
        } else {
            Vec::new()
        };

        // Collecte les logs et events du contexte
        let context = self.context.lock().unwrap();
        let logs = context.get_logs().clone();
        let events = context.get_events().clone();
        let state_changes = context.get_state_changes().clone();

        Ok(ExecutionResult {
            return_data,
            gas_used,
            logs,
            events,
            state_changes,
            success: true,
            error_message: None,
        })
    }

    /// Lit des données de la mémoire WASM
    fn read_memory(&self, ptr: u32, len: u32) -> ContractResult<Vec<u8>> {
        let memory = self.instance.exports.get_memory("memory")
            .map_err(|e| ContractError::WasmExecution { 
                message: format!("Memory access failed: {}", e) 
            })?;

        let memory_view = memory.view(&self.store);
        let mut data = vec![0u8; len as usize];
        
        for (i, cell) in memory_view[ptr as usize..(ptr + len) as usize].iter().enumerate() {
            data[i] = cell.get();
        }

        Ok(data)
    }

    /// Écrit des données dans la mémoire WASM
    fn write_memory(&mut self, ptr: u32, data: &[u8]) -> ContractResult<()> {
        let memory = self.instance.exports.get_memory("memory")
            .map_err(|e| ContractError::WasmExecution { 
                message: format!("Memory access failed: {}", e) 
            })?;

        let memory_view = memory.view(&self.store);
        
        for (i, &byte) in data.iter().enumerate() {
            memory_view[(ptr as usize) + i].set(byte);
        }

        Ok(())
    }

    /// Alloue de la mémoire dans le contrat WASM
    pub fn allocate_memory(&mut self, size: u32) -> ContractResult<u32> {
        // Appelle la fonction d'allocation du contrat si elle existe
        if let Ok(alloc_fn) = self.instance.exports.get_function("alloc") {
            let result = alloc_fn.call(&mut self.store, &[Value::I32(size as i32)])
                .map_err(|e| ContractError::WasmExecution { 
                    message: format!("Memory allocation failed: {}", e) 
                })?;

            if let Some(Value::I32(ptr)) = result.get(0) {
                Ok(*ptr as u32)
            } else {
                Err(ContractError::WasmExecution { 
                    message: "Invalid allocation result".to_string() 
                })
            }
        } else {
            Err(ContractError::FunctionNotFound { 
                function: "alloc".to_string() 
            })
        }
    }

    /// Libère de la mémoire dans le contrat WASM
    pub fn deallocate_memory(&mut self, ptr: u32) -> ContractResult<()> {
        if let Ok(dealloc_fn) = self.instance.exports.get_function("dealloc") {
            dealloc_fn.call(&mut self.store, &[Value::I32(ptr as i32)])
                .map_err(|e| ContractError::WasmExecution { 
                    message: format!("Memory deallocation failed: {}", e) 
                })?;
            Ok(())
        } else {
            // Si pas de fonction de désallocation, on ignore silencieusement
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::ContextProvider;

    #[test]
    fn test_runtime_creation() {
        let config = WasmRuntimeConfig::default();
        let runtime = WasmRuntime::new(config);
        assert!(runtime.is_ok());
    }

    #[test]
    fn test_module_compilation() {
        let config = WasmRuntimeConfig::default();
        let mut runtime = WasmRuntime::new(config).unwrap();
        
        // Bytecode WASM minimal (module vide)
        let wat = r#"
            (module
                (func (export "test") (result i32)
                    i32.const 42
                )
            )
        "#;
        
        let wasm_bytes = wat::parse_str(wat).unwrap();
        let contract_hash = Hash::zero();
        
        let result = runtime.compile_module(&wasm_bytes, contract_hash);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execution_result_serialization() {
        let result = ExecutionResult {
            return_data: vec![1, 2, 3],
            gas_used: 1000,
            logs: vec!["test log".to_string()],
            events: vec![],
            state_changes: vec![],
            success: true,
            error_message: None,
        };

        let serialized = serde_json::to_string(&result).unwrap();
        let deserialized: ExecutionResult = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(result.return_data, deserialized.return_data);
        assert_eq!(result.gas_used, deserialized.gas_used);
        assert_eq!(result.success, deserialized.success);
    }
}