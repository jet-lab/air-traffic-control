# Air Traffic Control

Solana validator proxy for simulating RPC traffic downtime and errors.

## Configuration

By default, the binary looks for an `ATC_CONFIG_PATH` environment variable to find where the tool's configuration file exists in the file system.

> If the variable is not set or there is an error parsing the file, a default configuration will be instantiated for the application to run.

| Name                     |   Type   |                               Description                                |         Default         |
| :----------------------- | :------: | :----------------------------------------------------------------------: | :---------------------: |
| `rpcEndpoint`            | `string` |     The URL of the RPC endpoint whose traffic the proxy is fronting.     | `http://localhost:8899` |
| `percentages`            | `object` |           Configuration of RPC and transaction success rates.            |            -            |
| `percentages.rpcSuccess` | `float`  |     A decimal from 0-1 to symbolize success percentage of RPC calls.     |         `0.65`          |
| `percentages.txSuccess`  | `float`  | A decimal from 0-1 to symbolize success percentage of sent transactions. |          `0.8`          |
| `port`                   |  `int`   |               The port number for the proxy to listen on.                |         `8080`          |
| `workers`                |  `int`   |         Number of parallel workers for the proxy server to run.          |          `10`           |

### Example

```json
{
  "rpcEndpoint": "http://localhost:8899",
  "percentages": {
    "rpcSuccess": 1.0,
    "txSuccess": 0.5
  },
  "port": 8080,
  "workers": 10
}
```
