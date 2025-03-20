# Ethereum Gas Estimation Service

A high-performance gas estimation service for Ethereum transactions written in Rust.

## Features

- Accurate gas estimations for Ethereum transactions using REVM simulation
- Full support for all transaction types:
  - Legacy (Type 0) transactions
  - EIP-2930 (Type 1) transactions with access lists
  - EIP-1559 (Type 2) transactions with dynamic fee market
  - EIP-4844 (Type 3) blob transactions
  - EIP-7702 (Type 4) authorization list transactions
- JSON-RPC compatible API with full eth_estimateGas support
- Comprehensive logging for debugging and monitoring
- Built with Rust and modern Ethereum tooling (Alloy, Anvil for local tests, REVM, Foundry) for high performance and memory safety

## Prerequisites

- Rust (1.85+)
- Access to an Ethereum node or service provider (Infura, Alchemy, etc.)

## Setup

1. Clone the repository:

```bash
git clone --recurse-submodules https://github.com/giaki3003/eth-gas-estimator.git
cd eth-gas-estimator
```
Make sure you clone with `--recurse-submodules` to pull in the Foundry submodule.

2. Create a `.env` file from the template:

```bash
cp .env.example .env
```

3. Configure your environment variables in the `.env` file:

```
HOST=127.0.0.1
PORT=8080
ETHEREUM_RPC_URL=https://mainnet.infura.io/v3/YOUR_INFURA_KEY
```

4. Build the project:

```bash
cargo build --release
```

## Running the Service

Start the service:

```bash
cargo run --release
```

The service will be available at `http://127.0.0.1:8080` (or the host/port you configured).

## How Gas Estimation Works

The service uses REVM (Rust Ethereum Virtual Machine) with Foundry-EVM's SharedBackend to simulate transaction execution:

**REVM Simulation**: The transaction is executed on a fork of the current Ethereum network state, providing precise measurement of the actual gas used.

**NOTICE:** Our estimates will typically differ from traditional go-ethereum "estimateGas" RPC calls. Go-ethereum uses a [binary search approach](https://github.com/ethereum/go-ethereum/blob/80b8d7a13c20254a9cfb9f7cbca1ab00aa6a3b50/eth/gasestimator/gasestimator.go#L55) between 21,000 (minimum gas) and the gas limit to approximate gas usage. Our REVM approach actually executes the transaction in a simulation environment, providing a more accurate result. This lets us maintain API compatibility while offering superior estimation.

## API Documentation

### Estimate Gas

**Endpoint:** `POST /api/v1/eth/estimateGas`

The API follows the standard Ethereum JSON-RPC format for compatibility with existing tools and libraries.

#### Example Requests

**1. Legacy (Type 0) Transaction:**

```json
{
  "jsonrpc": "2.0",
  "method": "eth_estimateGas",
  "params": [{
    "from": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
    "to": "0x742d35Cc6634C0532925a3b844Bc454e4438f44f",
    "gasPrice": "0x4a817c800",
    "value": "0xde0b6b3a7640000"
  }],
  "id": 1
}
```

**2. EIP-2930 (Type 1) Transaction with Access List:**

```json
{
  "jsonrpc": "2.0",
  "method": "eth_estimateGas",
  "params": [{
    "from": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
    "to": "0x742d35Cc6634C0532925a3b844Bc454e4438f44f",
    "gasPrice": "0x4a817c800",
    "value": "0xde0b6b3a7640000",
    "accessList": [
      {
        "address": "0x742d35Cc6634C0532925a3b844Bc454e4438f44f",
        "storageKeys": [
          "0x0000000000000000000000000000000000000000000000000000000000000001"
        ]
      }
    ],
    "type": "0x1"
  }],
  "id": 1
}
```

**3. EIP-1559 (Type 2) Transaction:**

```json
{
  "jsonrpc": "2.0",
  "method": "eth_estimateGas",
  "params": [{
    "from": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
    "to": "0x742d35Cc6634C0532925a3b844Bc454e4438f44f",
    "maxFeePerGas": "0x4a817c800",
    "maxPriorityFeePerGas": "0x3b9aca00",
    "value": "0xde0b6b3a7640000",
    "type": "0x2"
  }],
  "id": 1
}
```

**4. EIP-4844 (Type 3) Blob Transaction:**

```json
{
  "jsonrpc": "2.0",
  "method": "eth_estimateGas",
  "params": [{
    "from": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
    "to": "0x742d35Cc6634C0532925a3b844Bc454e4438f44f",
    "maxFeePerGas": "0x4a817c800",
    "maxPriorityFeePerGas": "0x3b9aca00",
    "maxFeePerBlobGas": "0x5f5e100",
    "blobVersionedHashes": [
      "0x0100000000000000000000000000000000000000000000000000000000000001"
    ],
    "type": "0x3"
  }],
  "id": 1
}
```

**5. EIP-7702 (Type 4) Authorization List Transaction:**

```json
{
  "jsonrpc": "2.0",
  "method": "eth_estimateGas",
  "params": [{
    "from": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
    "to": "0x742d35Cc6634C0532925a3b844Bc454e4438f44f",
    "maxFeePerGas": "0x4a817c800",
    "maxPriorityFeePerGas": "0x3b9aca00",
    "value": "0x0",
    "authorizationList": [
      {
        "chainId": "0x1",
        "contractAddress": "0x742d35Cc6634C0532925a3b844Bc454e4438f55e",
        "nonce": "0x1",
        "yParity": "0x0",
        "r": "0x8626f6940e2eb28930092b2e2c594f072f3503a7198105a320731219154aa7f4",
        "s": "0x2d55741fd8310643fd4683ddf8e72945441c605bd1c6c8fae3cb9aa78b598657"
      }
    ],
    "type": "0x04"
  }],
  "id": 1
}
```

**6. Contract Deployment:**

```json
{
  "jsonrpc": "2.0",
  "method": "eth_estimateGas",
  "params": [{
    "from": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
    "gas" : "0x493e0", 
    "data": "0x608060405234801561001057600080fd5b5060c78061001f6000396000f3fe6080604052348015600f57600080fd5b506004361060325760003560e01c806360fe47b11460375780636d4ce63c146062575b600080fd5b606060048036036020811015604b57600080fd5b8101908080359060200190929190505050607e565b005b60686088565b6040518082815260200191505060405180910390f35b8060008190555050565b6000805490509056fea264697066735822122018e873e978df16c207f8f6ed18612b17e2c2a70d0916ff978c0755f6a45e26fc64736f6c634300060c0033"
  }],
  "id": 1
}
```

**Response Format:**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": "0x5208"
}
```

### Health Check

**Endpoint:** `POST /api/v1/health`

**Response:**

```json
{
  "status": "ok",
  "latest_block": 15000000,
  "timestamp": 1650000000
}
```

## Error Codes

| Error Code | Description |
|------------|-------------|
| RPC_CONNECTION_ERROR | Cannot connect to Ethereum node |
| SIMULATION_ERROR | Transaction simulation failed |
| ESTIMATION_ERROR | Failed to estimate gas |

## Performance

- The REVM simulation approach offers highly accurate gas estimates, typically within 98% of actual on-chain gas usage
- Simulation occurs locally, eliminating additional RPC roundtrips
- Built with Rust for optimal performance and memory safety

## Testing

Run unit tests:

```bash
cargo test
```

Tests run with a local Anvil node for accurate simulation results.
Make sure you have it installed and in your PATH.

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/your-feature`)
3. Commit your changes (`git commit -am 'Add some feature'`)
4. Push to the branch (`git push origin feature/your-feature`)
5. Create a new Pull Request