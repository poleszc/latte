#[cfg(test)]
mod semantic_tests {
    use crate::parser;
    use crate::semantics::SemanticChecker;

    #[test]
    fn test_simple_program() {
        let source = r#"
            int main() {
                printInt(42);
                return 0;
            }
        "#;

        let program = parser::parse(source).expect("Parse failed");
        let result = SemanticChecker::check(&program);
        assert!(result.is_ok(), "Semantic check should pass");
    }

    #[test]
    fn test_undefined_variable() {
        let source = r#"
            int main() {
                printInt(x);
                return 0;
            }
        "#;

        let program = parser::parse(source).expect("Parse failed");
        let result = SemanticChecker::check(&program);
        assert!(result.is_err(), "Should detect undefined variable");
    }

    #[test]
    fn test_type_mismatch() {
        let source = r#"
            int main() {
                int x;
                x = true;
                return 0;
            }
        "#;

        let program = parser::parse(source).expect("Parse failed");
        let result = SemanticChecker::check(&program);
        assert!(result.is_err(), "Should detect type mismatch");
    }

    #[test]
    fn test_function_with_args() {
        let source = r#"
            int add(int a, int b) {
                return a + b;
            }

            int main() {
                int result;
                result = add(5, 3);
                return 0;
            }
        "#;

        let program = parser::parse(source).expect("Parse failed");
        let result = SemanticChecker::check(&program);
        assert!(result.is_ok(), "Semantic check should pass");
    }

    #[test]
    fn test_wrong_argument_count() {
        let source = r#"
            int add(int a, int b) {
                return a + b;
            }

            int main() {
                int result;
                result = add(5);
                return 0;
            }
        "#;

        let program = parser::parse(source).expect("Parse failed");
        let result = SemanticChecker::check(&program);
        assert!(result.is_err(), "Should detect wrong argument count");
    }

    #[test]
    fn test_class_definition() {
        let source = r#"
            class Point {
                int x;
                int y;
                
                void setX(int newX) {
                    x = newX;
                }
                
                int getX() {
                    return x;
                }
            }

            int main() {
                Point p;
                p = new Point;
                p.setX(10);
                int val;
                val = p.getX();
                return 0;
            }
        "#;

        let program = parser::parse(source).expect("Parse failed");
        let result = SemanticChecker::check(&program);
        assert!(result.is_ok(), "Semantic check should pass");
    }

    #[test]
    fn test_class_inheritance() {
        let source = r#"
            class Animal {
                string name;
                
                void setName(string n) {
                    name = n;
                }
            }

            class Dog extends Animal {
                void bark() {
                    printString("Woof!");
                }
            }

            int main() {
                Dog d;
                d = new Dog;
                d.setName("Rex");
                d.bark();
                return 0;
            }
        "#;

        let program = parser::parse(source).expect("Parse failed");
        let result = SemanticChecker::check(&program);
        assert!(result.is_ok(), "Semantic check should pass");
    }

    #[test]
    fn test_array_operations() {
        let source = r#"
            int main() {
                int[] arr;
                arr = new int[10];
                arr[0] = 5;
                int x;
                x = arr[0];
                return 0;
            }
        "#;

        let program = parser::parse(source).expect("Parse failed");
        let result = SemanticChecker::check(&program);
        assert!(result.is_ok(), "Semantic check should pass");
    }

    #[test]
    fn test_for_each_loop() {
        let source = r#"
            int main() {
                int[] arr;
                arr = new int[5];
                
                for (int x : arr) {
                    printInt(x);
                }
                
                return 0;
            }
        "#;

        let program = parser::parse(source).expect("Parse failed");
        let result = SemanticChecker::check(&program);
        assert!(result.is_ok(), "Semantic check should pass");
    }

    #[test]
    fn test_zero_division() {
        let source = r#"
            int main() {
                int x;
                x = 10 / 0;
                return 0;
            }
        "#;

        let program = parser::parse(source).expect("Parse failed");
        let result = SemanticChecker::check(&program);
        assert!(result.is_err(), "Should detect division by zero");
    }

    #[test]
    fn test_return_type_mismatch() {
        let source = r#"
            int foo() {
                return true;
            }

            int main() {
                return 0;
            }
        "#;

        let program = parser::parse(source).expect("Parse failed");
        let result = SemanticChecker::check(&program);
        assert!(result.is_err(), "Should detect return type mismatch");
    }

    #[test]
    fn test_no_main_function() {
        let source = r#"
            int foo() {
                return 42;
            }
        "#;

        let program = parser::parse(source).expect("Parse failed");
        let result = SemanticChecker::check(&program);
        assert!(result.is_err(), "Should detect missing main function");
    }

    #[test]
    fn test_duplicate_variable() {
        let source = r#"
            int main() {
                int x;
                int x;
                return 0;
            }
        "#;

        let program = parser::parse(source).expect("Parse failed");
        let result = SemanticChecker::check(&program);
        assert!(result.is_err(), "Should detect duplicate variable");
    }

    #[test]
    fn test_invalid_lvalue() {
        let source = r#"
            int main() {
                5 = 10;
                return 0;
            }
        "#;

        let program = parser::parse(source).expect("Parse failed");
        let result = SemanticChecker::check(&program);
        assert!(result.is_err(), "Should detect invalid lvalue");
    }

    #[test]
    fn test_boolean_operations() {
        let source = r#"
            int main() {
                boolean a;
                boolean b;
                a = true;
                b = false;
                
                if (a && b) {
                    printString("Both true");
                }
                
                if (a || b) {
                    printString("At least one true");
                }
                
                if (!a) {
                    printString("a is false");
                }
                
                return 0;
            }
        "#;

        let program = parser::parse(source).expect("Parse failed");
        let result = SemanticChecker::check(&program);
        assert!(result.is_ok(), "Semantic check should pass");
    }

    #[test]
    fn test_string_comparison() {
        let source = r#"
            int main() {
                string s1;
                string s2;
                s1 = "hello";
                s2 = "world";
                
                if (s1 == s2) {
                    printString("Equal");
                }
                
                return 0;
            }
        "#;

        let program = parser::parse(source).expect("Parse failed");
        let result = SemanticChecker::check(&program);
        assert!(result.is_ok(), "Semantic check should pass");
    }
}
