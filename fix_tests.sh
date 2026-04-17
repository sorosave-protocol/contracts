#!/bin/bash
# Add description parameter after name in all create_group calls
sed -i '' 's/String::from_str(&env, "\([^"]*\)"),$/String::from_str(\&env, "\1"),\n        \&String::from_str(\&env, "Test description"),/g' contracts/sorosave/src/test.rs
