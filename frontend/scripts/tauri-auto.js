#!/usr/bin/env node
/**
 * Auto-detect GPU and run Tauri with appropriate features
 */

const { execSync } = require('child_process');
const path = require('path');

// Get the command (dev or build)
const command = process.argv[2];
if (!command || !['dev', 'build'].includes(command)) {
  console.error('Usage: node tauri-auto.js [dev|build]');
  process.exit(1);
}

// Detect GPU feature
let feature = '';
try {
  const result = execSync('node scripts/auto-detect-gpu.js', {
    encoding: 'utf8',
    stdio: ['pipe', 'pipe', 'inherit']
  });
  feature = result.trim();
} catch (err) {
  // If detection fails, continue with no features
}

console.log(''); // Empty line for spacing

// Build llama-helper with matching GPU features
console.log('🦙 Building llama-helper sidecar...');
const helperDir = path.join(__dirname, '../../llama-helper');
const helperFeatures = feature ? `--features ${feature}` : '';
const helperCmd = `cd ${helperDir} && cargo build --release ${helperFeatures}`;

try {
  execSync(helperCmd, { stdio: 'inherit' });
  console.log('✅ llama-helper built successfully');
} catch (err) {
  console.error('❌ Failed to build llama-helper');
  process.exit(1);
}

// Detect target triple for proper sidecar naming
let targetTriple = '';
try {
  const rustcOutput = execSync('rustc -vV', { encoding: 'utf8' });
  const hostMatch = rustcOutput.match(/host:\s*(\S+)/);
  if (hostMatch) {
    targetTriple = hostMatch[1];
  }
} catch (err) {
  console.error('❌ Failed to detect Rust target triple');
  process.exit(1);
}

console.log(`🎯 Target triple: ${targetTriple}`);

// Copy binary to src-tauri/binaries/ with target triple suffix
const fs = require('fs');
const binariesDir = path.join(__dirname, '../src-tauri/binaries');
if (!fs.existsSync(binariesDir)) {
  fs.mkdirSync(binariesDir, { recursive: true });
}

const platform = require('os').platform();
const baseBinary = platform === 'win32' ? 'llama-helper.exe' : 'llama-helper';
const sidecarBinary = platform === 'win32'
  ? `llama-helper-${targetTriple}.exe`
  : `llama-helper-${targetTriple}`;

const srcPath = path.join(helperDir, '../target/release', baseBinary);
const destPath = path.join(binariesDir, sidecarBinary);

if (fs.existsSync(srcPath)) {
  fs.copyFileSync(srcPath, destPath);
  console.log(`✅ Copied llama-helper to ${destPath}`);
} else {
  console.error(`❌ llama-helper binary not found at ${srcPath}`);
  process.exit(1);
}

console.log('');

// Build the tauri command
let tauriCmd = `tauri ${command}`;
if (feature) {
  tauriCmd += ` -- --features ${feature}`;
  console.log(`🚀 Running: tauri ${command} with features: ${feature}`);
} else {
  console.log(`🚀 Running: tauri ${command} (CPU-only mode)`);
}
console.log('');

// Execute the command
try {
  execSync(tauriCmd, { stdio: 'inherit' });
} catch (err) {
  process.exit(err.status || 1);
}
