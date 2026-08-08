// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

interface IERC20 {
    function transfer(address to, uint256 amount) external returns (bool);
    function approve(address spender, uint256 amount) external returns (bool);
    function balanceOf(address account) external view returns (uint256);
}

interface IAavePool {
    function flashLoanSimple(
        address receiverAddress,
        address asset,
        uint256 amount,
        bytes calldata params,
        uint16 referralCode
    ) external;
}

interface IUniswapV2Router {
    function swapExactTokensForTokens(
        uint amountIn,
        uint amountOutMin,
        address[] calldata path,
        address to,
        uint deadline
    ) external returns (uint[] memory amounts);
    function getAmountsOut(uint amountIn, address[] calldata path)
        external view returns (uint[] memory amounts);
}

contract FlashArb {
    address public constant USDC = 0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174;
    address public constant WETH = 0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619;
    address public constant AAVE_POOL = 0x4EA9bC7d5e445B7a3bDc6F2d82f1B95C0F80A6D1;

    IUniswapV2Router public buyRouter;
    IUniswapV2Router public sellRouter;

    address public owner;
    bool public paused;

    uint256 public minProfit = 0.05 * 1e6;
    uint256 public maxSlippageBps = 30;
    uint256 public maxLoanAmount = 5000 * 1e6;

    event ArbExecuted(uint256 profit, uint256 amountIn, address initiator);
    event ArbReverted(string reason);
    event RiskParamsUpdated();

    modifier onlyOwner() {
        require(msg.sender == owner, "Not owner");
        _;
    }

    modifier whenNotPaused() {
        require(!paused, "Paused");
        _;
    }

    constructor(address _buyRouter, address _sellRouter) {
        owner = msg.sender;
        buyRouter = IUniswapV2Router(_buyRouter);
        sellRouter = IUniswapV2Router(_sellRouter);
    }

    function executeFlashLoan(uint256 amount) external onlyOwner whenNotPaused {
        require(amount <= maxLoanAmount, "Loan too large");
        IAavePool(AAVE_POOL).flashLoanSimple(
            address(this),
            USDC,
            amount,
            "",
            0
        );
    }

    // ---- Simplified executeOperation (stack-safe) ----
    function executeOperation(
        address asset,
        uint256 amount,
        uint256 premium,
        address initiator,
        bytes calldata
    ) external returns (bool) {
        require(msg.sender == AAVE_POOL, "Not Aave pool");
        require(initiator == address(this), "Bad initiator");

        // 1. Buy WETH with USDC
        uint256 wethReceived = _buyWeth(amount);

        // 2. Sell WETH for USDC
        uint256 usdcReceived = _sellWeth(wethReceived);

        // 3. Repay and profit
        return _settle(amount, premium, usdcReceived, initiator);
    }

    // ---- Internal: Buy leg ----
    function _buyWeth(uint256 amount) internal returns (uint256) {
        address[] memory path = new address[](2);
        path[0] = USDC;
        path[1] = WETH;

        uint256[] memory expected = buyRouter.getAmountsOut(amount, path);
        uint256 minOut = expected[1] - (expected[1] * maxSlippageBps / 10000);

        IERC20(USDC).approve(address(buyRouter), amount);
        uint256[] memory result = buyRouter.swapExactTokensForTokens(
            amount, minOut, path, address(this), block.timestamp
        );
        return result[1];
    }

    // ---- Internal: Sell leg ----
    function _sellWeth(uint256 wethAmount) internal returns (uint256) {
        address[] memory path = new address[](2);
        path[0] = WETH;
        path[1] = USDC;

        uint256[] memory expected = sellRouter.getAmountsOut(wethAmount, path);
        uint256 minOut = expected[1] - (expected[1] * maxSlippageBps / 10000);

        IERC20(WETH).approve(address(sellRouter), wethAmount);
        uint256[] memory result = sellRouter.swapExactTokensForTokens(
            wethAmount, minOut, path, address(this), block.timestamp
        );
        return result[1];
    }

    // ---- Internal: Settle loan and capture profit ----
    function _settle(
        uint256 amount,
        uint256 premium,
        uint256 usdcReceived,
        address initiator
    ) internal returns (bool) {
        uint256 amountOwed = amount + premium;

        require(usdcReceived >= amountOwed + minProfit, "Profit too low");

        IERC20(USDC).approve(AAVE_POOL, amountOwed);
        uint256 profit = usdcReceived - amountOwed;
        IERC20(USDC).transfer(owner, profit);

        emit ArbExecuted(profit, amount, initiator);
        return true;
    }

    // ---- Owner Controls ----
    function setRouters(address _buy, address _sell) external onlyOwner {
        buyRouter = IUniswapV2Router(_buy);
        sellRouter = IUniswapV2Router(_sell);
        emit RiskParamsUpdated();
    }

    function setRiskParams(
        uint256 _minProfit,
        uint256 _maxSlippageBps,
        uint256 _maxLoanAmount
    ) external onlyOwner {
        require(_maxSlippageBps <= 500, "Slippage too loose");
        minProfit = _minProfit;
        maxSlippageBps = _maxSlippageBps;
        maxLoanAmount = _maxLoanAmount;
        emit RiskParamsUpdated();
    }

    function pause() external onlyOwner { paused = true; }
    function unpause() external onlyOwner { paused = false; }

    function withdrawTokens(address token, uint256 amount) external onlyOwner {
        IERC20(token).transfer(owner, amount);
    }

    receive() external payable {}
}