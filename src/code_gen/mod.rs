pub mod error;
pub mod instruction;
pub mod log_byte_code;
pub mod service_byte_code;

#[cfg(test)]
mod tests {
    use crate::{
        code_gen::{
            instruction::{Instruction, StackValue},
            log_byte_code::LogByteCodeGenerator,
            service_byte_code::ServiceByteCodeGenerator,
        },
        config::{Call, Config, Count, Method, Service, Severity, Task},
    };

    #[test]
    fn test_config_parse() {
        let config = Config {
            logs: vec![Task {
                name: "test".to_string(),
                frequency: 1000,
                count: Count::Amount(10),
                template: "User logged in".to_string(),
                vars: vec![],
                severity: Severity::Info,
            }],
            services: vec![],
        };
        let generator = LogByteCodeGenerator::new(&config.logs[0]);
        let code = generator.process_task().unwrap();

        /*
        StoreVar("name", "test")              // Store task name
        StoreVar("template", "User logged in") // Store template
        Push(10)                              // Initial counter value
        Label("loop_start")                   // Loop start
        Dup                                   // Duplicate counter on stack
        JmpIfZero("loop_end")                 // Exit if counter is zero
        Dec                                   // Decrement the counter
        LoadVar("template")                   // Load template
        Stdout                                // Print to stdout
        Sleep(1000)                           // Wait 1 second
        Jump("loop_start")                    // Jump back to loop start
        Label("loop_end")                     // Loop end
        Pop                                   // Clean up counter from stack
        */

        assert_eq!(code.len(), 13);
        assert_eq!(
            code[0],
            Instruction::StoreVar("name".to_string(), "test".to_string())
        );
        assert_eq!(
            code[1],
            Instruction::StoreVar("template".to_string(), "User logged in".to_string())
        );
        assert_eq!(code[2], Instruction::Push(StackValue::Int(10)));
        assert_eq!(code[3], Instruction::Label("loop_test".to_string()));
        assert_eq!(code[4], Instruction::Dup);
        assert_eq!(code[5], Instruction::JmpIfZero("end_test".to_string()));
        assert_eq!(code[6], Instruction::Dec);
        assert_eq!(code[7], Instruction::LoadVar("template".to_string()));
        assert_eq!(code[8], Instruction::Stdout);
        assert_eq!(code[9], Instruction::Sleep(1000));
        assert_eq!(code[10], Instruction::Jump("loop_test".to_string()));
        assert_eq!(code[11], Instruction::Label("end_test".to_string()));
        assert_eq!(code[12], Instruction::Pop);
    }

    #[test]
    fn test_counted_loop_with_vars() {
        let config = Config {
            logs: vec![Task {
                name: "test".to_string(),
                frequency: 1000,
                count: Count::Amount(10),
                template: "User %s logged in".to_string(),
                vars: vec!["John".to_string()],
                severity: Severity::Info,
            }],
            services: vec![],
        };
        let generator = LogByteCodeGenerator::new(&config.logs[0]);
        let code = generator.process_task().unwrap();

        /*
        StoreVar("name", "test")              // Store task name
        StoreVar("template", "User logged in") // Store template
        Push(10)                              // Initial counter value
        Label("loop_start")                   // Loop start
        Dup                                   // Duplicate counter on stack
        JmpIfZero("loop_end")                 // Exit if counter is zero
        Dec                                   // Decrement the counter
        LoadVar("template")                   // Load template
        LoadVar("var_0")                      // Load variable
        Printf                                // Join the strings
        Stdout                                // Print to stdout
        Sleep(1000)                           // Wait 1 second
        Jump("loop_start")                    // Jump back to loop start
        Label("loop_end")                     // Loop end
        Pop                                   // Clean up counter from stack
        */
        assert_eq!(code.len(), 16);
        assert_eq!(
            code[0],
            Instruction::StoreVar("name".to_string(), "test".to_string())
        );
        assert_eq!(
            code[1],
            Instruction::StoreVar("template".to_string(), "User %s logged in".to_string())
        );
        assert_eq!(
            code[2],
            Instruction::StoreVar("var_0".to_string(), "John".to_string())
        );
        assert_eq!(code[3], Instruction::Push(StackValue::Int(10)));
        assert_eq!(code[4], Instruction::Label("loop_test".to_string()));
        assert_eq!(code[5], Instruction::Dup);
        assert_eq!(code[6], Instruction::JmpIfZero("end_test".to_string()));
        assert_eq!(code[7], Instruction::Dec);
        assert_eq!(code[8], Instruction::LoadVar("var_0".to_string()));
        assert_eq!(code[9], Instruction::LoadVar("template".to_string()));
        assert_eq!(code[10], Instruction::Printf);
        assert_eq!(code[11], Instruction::Stdout);
        assert_eq!(code[12], Instruction::Sleep(1000));
        assert_eq!(code[13], Instruction::Jump("loop_test".to_string()));
        assert_eq!(code[14], Instruction::Label("end_test".to_string()));
        assert_eq!(code[15], Instruction::Pop);
    }

    #[test]
    fn test_generate_infinite_loop_with_single_var() {
        let config = Config {
            logs: vec![Task {
                name: "test".to_string(),
                frequency: 1000,
                count: Count::Const("Infinite".to_string()),
                template: "User %s logged in".to_string(),
                vars: vec!["John".to_string()],
                severity: Severity::Info,
            }],
            services: vec![],
        };
        let generator = LogByteCodeGenerator::new(&config.logs[0]);
        let code = generator.process_task().unwrap();

        /*
        StoreVar("name", "test")              // Store task name
        StoreVar("template", "User %s logged in") // Store template
        StoreVar("var_0", "John")               // Store variable
        Label("loop_start")                   // Loop start
        LoadVar("var_0")                      // Load variable
        LoadVar("template")                   // Load template
        Printf                                // Join the strings
        Stdout                                // Print to stdout
        Sleep(1000)                           // Wait 1 second
        Jump("loop_start")                    // Jump back to loop start
        Label("loop_end")                     // Loop end
        */

        assert_eq!(code.len(), 11);
        assert_eq!(
            code[0],
            Instruction::StoreVar("name".to_string(), "test".to_string())
        );
        assert_eq!(
            code[1],
            Instruction::StoreVar("template".to_string(), "User %s logged in".to_string())
        );
        assert_eq!(
            code[2],
            Instruction::StoreVar("var_0".to_string(), "John".to_string())
        );
        assert_eq!(code[3], Instruction::Label("loop_test".to_string()));
        assert_eq!(code[4], Instruction::LoadVar("var_0".to_string()));
        assert_eq!(code[5], Instruction::LoadVar("template".to_string()));
        assert_eq!(code[6], Instruction::Printf);
        assert_eq!(code[7], Instruction::Stdout);
        assert_eq!(code[8], Instruction::Sleep(1000));
        assert_eq!(code[9], Instruction::Jump("loop_test".to_string()));
        assert_eq!(code[10], Instruction::Label("end_test".to_string()));
    }

    #[test]
    fn test_generate_infinite_loop() {
        let config = Config {
            logs: vec![Task {
                name: "test".to_string(),
                frequency: 1000,
                count: Count::Const("Infinite".to_string()),
                template: "User logged in".to_string(),
                vars: vec![],
                severity: Severity::Info,
            }],
            services: vec![],
        };
        let generator = LogByteCodeGenerator::new(&config.logs[0]);
        let code = generator.process_task().unwrap();

        /*
        StoreVar("name", "test")              // Store task name
        StoreVar("template", "User logged in") // Store template
        Label("loop_start")                   // Loop start
        LoadVar("template")                   // Load template
        Stdout                                // Print to stdout
        Sleep(1000)                           // Wait 1 second
        Jump("loop_start")                    // Jump back to loop start
        Label("loop_end")                     // Loop end
        */

        assert_eq!(code.len(), 8);
        assert_eq!(
            code[0],
            Instruction::StoreVar("name".to_string(), "test".to_string())
        );
        assert_eq!(
            code[1],
            Instruction::StoreVar("template".to_string(), "User logged in".to_string())
        );
        assert_eq!(code[2], Instruction::Label("loop_test".to_string()));
        assert_eq!(code[3], Instruction::LoadVar("template".to_string()));
        assert_eq!(code[4], Instruction::Stdout);
        assert_eq!(code[5], Instruction::Sleep(1000));
        assert_eq!(code[6], Instruction::Jump("loop_test".to_string()));
        assert_eq!(code[7], Instruction::Label("end_test".to_string()));
    }

    #[test]
    fn test_print_stderr() {
        let config = Config {
            logs: vec![Task {
                name: "test".to_string(),
                frequency: 1000,
                count: Count::Const("Infinite".to_string()),
                template: "User logged in".to_string(),
                vars: vec![],
                severity: Severity::Error,
            }],
            services: vec![],
        };
        let generator = LogByteCodeGenerator::new(&config.logs[0]);
        let code = generator.process_task().unwrap();

        /*
        StoreVar("name", "test")              // Store task name
        StoreVar("template", "User logged in") // Store template
        Label("loop_start")                   // Loop start
        LoadVar("name")                       // Load the name (was "test")
        Push(" ")                             // Push separator
        LoadVar("template")                   // Load template
        StrJoin                               // Join the strings
        StdErr                                // Print to stderr
        Sleep(1000)                           // Wait 1 second
        Jump("loop_start")                    // Jump back to loop start
        Label("loop_end")                     // Loop end
        */

        assert_eq!(code.len(), 8);
        assert_eq!(
            code[0],
            Instruction::StoreVar("name".to_string(), "test".to_string())
        );
        assert_eq!(
            code[1],
            Instruction::StoreVar("template".to_string(), "User logged in".to_string())
        );
        assert_eq!(code[2], Instruction::Label("loop_test".to_string()));
        assert_eq!(code[3], Instruction::LoadVar("template".to_string()));
        assert_eq!(code[4], Instruction::Stderr);
        assert_eq!(code[5], Instruction::Sleep(1000));
        assert_eq!(code[6], Instruction::Jump("loop_test".to_string()));
        assert_eq!(code[7], Instruction::Label("end_test".to_string()));
    }

    #[test]
    fn test_generate_services() {
        let config = Config {
            logs: vec![],
            services: vec![Service {
                name: "test".to_string(),
                invoke: Some(vec!["charge".to_string()]),
                methods: vec![Method {
                    name: "charge".to_string(),
                    stdout: Some("Charging".to_string()),
                    sleep_ms: Some(500),
                    calls: Some(vec![Call {
                        name: "checkout".to_string(),
                        method: "process".to_string(),
                    }]),
                }],
            }],
        };

        let generator = ServiceByteCodeGenerator::new(&config.services[0]);
        let code = generator.process_service().unwrap();

        /*
        StoreVar("name", "test")              // Store task name
        Jump("main")
        ---
        Label("charge")
        Push("Charging")
        Stdout
        Sleep(500)
        Label("end_charge")
        Push("checkout")
        Push("process")
        RemoteCall
        Jump("main")
        ---
        Label("main")
        Jump("charge")
        Jump("main")
        Label("end_main")
        */
        assert_eq!(code.len(), 14);
        assert_eq!(
            code[0],
            Instruction::StoreVar("name".to_string(), "test".to_string())
        );
        assert_eq!(code[1], Instruction::Jump("main".to_string()));
        //--
        assert_eq!(code[2], Instruction::Label("charge".to_string()));
        assert_eq!(
            code[3],
            Instruction::Push(StackValue::String("Charging".to_string()))
        );
        assert_eq!(code[4], Instruction::Stdout);
        assert_eq!(code[5], Instruction::Sleep(500));
        assert_eq!(
            code[6],
            Instruction::Push(StackValue::String("checkout".to_string()))
        );
        assert_eq!(
            code[7],
            Instruction::Push(StackValue::String("process".to_string()))
        );
        assert_eq!(code[8], Instruction::RemoteCall);
        assert_eq!(code[9], Instruction::Jump("main".to_string()));
        assert_eq!(code[10], Instruction::Label("end_charge".to_string()));
        //--
        assert_eq!(code[11], Instruction::Label("main".to_string()));
        assert_eq!(code[12], Instruction::Jump("charge".to_string()));
        assert_eq!(code[13], Instruction::Label("end_main".to_string()));
    }

    #[test]
    fn test_generate_services_without_invoke() {
        let config = Config {
            logs: vec![],
            services: vec![Service {
                name: "test".to_string(),
                invoke: None,
                methods: vec![Method {
                    name: "charge".to_string(),
                    stdout: Some("Charging".to_string()),
                    sleep_ms: Some(500),
                    calls: None,
                }],
            }],
        };

        let generator = ServiceByteCodeGenerator::new(&config.services[0]);
        let code = generator.process_service().unwrap();

        /*
        StoreVar("name", "test")              // Store task name
        Jump("main")
        Label("charge")
        Push("Charging")
        Stdout
        Sleep(500)
        Label("end_charge")
        Jump("charge")
        Label("end_main")
        */
        assert_eq!(code.len(), 10);
        assert_eq!(
            code[0],
            Instruction::StoreVar("name".to_string(), "test".to_string())
        );
        assert_eq!(code[1], Instruction::Jump("main".to_string()));
        assert_eq!(code[2], Instruction::Label("charge".to_string()));
        assert_eq!(
            code[3],
            Instruction::Push(StackValue::String("Charging".to_string()))
        );
        assert_eq!(code[4], Instruction::Stdout);
        assert_eq!(code[5], Instruction::Sleep(500));
        assert_eq!(code[6], Instruction::Jump("main".to_string()));
        assert_eq!(code[7], Instruction::Label("end_charge".to_string()));
        assert_eq!(code[8], Instruction::Label("main".to_string()));
        assert_eq!(code[9], Instruction::Label("end_main".to_string()));
    }
}
