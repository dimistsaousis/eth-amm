use ethers::prelude::abigen;

abigen!(
    GetWethValueInPoolBatchRequest,
    "./out/GetWethValueInPoolBatchRequest.sol/GetWethValueInPoolBatchRequest.json";

    IUniswapV2Pair,
    r#"[
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)
        function token0() external view returns (address)
        function token1() external view returns (address)
        function factory() external view returns (address)
        function swap(uint256 amount0Out, uint256 amount1Out, address to, bytes calldata data);
        event Sync(uint112 reserve0, uint112 reserve1)
    ]"#;

    IErc20,
    r#"[
        function balanceOf(address account) external view returns (uint256)
        function decimals() external view returns (uint8)
    ]"#;

    GetUniswapV2PoolDataBatchRequest,
    "./out/GetUniswapV2PoolDataBatchRequest.sol/GetUniswapV2PoolDataBatchRequest.json";

    GetUniswapV2PairsBatchRequest,
    "./out/GetUniswapV2PairsBatchRequest.sol/GetUniswapV2PairsBatchRequest.json";

    SimulatorV1,
    "./out/SimulatorV1.sol/SimulatorV1.json";

    IUniswapV2Factory,
    r#"[
        function getPair(address tokenA, address tokenB) external view returns (address pair)
        function allPairs(uint256 index) external view returns (address)
        event PairCreated(address indexed token0, address indexed token1, address pair, uint256)
        function allPairsLength() external view returns (uint256)

    ]"#;

    IUniswapRouter,
    r#"[
        function swapExactETHForTokens(uint amountOutMin, address[] calldata path, address to, uint deadline) external payable returns (uint[] memory amounts)
        ]"#;

);
