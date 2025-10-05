const invoke = window.__TAURI__.core.invoke;

let blockchainData = [];

async function loadBlockchain() {
    try {
        blockchainData = await invoke('get_blockchain');
        renderBlockchain();
        updateStats();
    } catch (error) {
        console.error('Failed to load blockchain:', error);
    }
}

async function addBlock() {
    const input = document.getElementById('blockData');
    const data = input.value.trim();
    
    if (!data) {
        alert('Please enter some data for the block');
        return;
    }

    const modal = document.getElementById('miningModal');
    const btn = document.getElementById('addBlockBtn');
    
    btn.disabled = true;
    modal.classList.add('active');

    try {
        blockchainData = await invoke('add_block', { data });
        input.value = '';
        renderBlockchain();
        updateStats();
    } catch (error) {
        console.error('Failed to add block:', error);
        alert('Failed to add block: ' + error);
    } finally {
        modal.classList.remove('active');
        btn.disabled = false;
    }
}

function renderBlockchain() {
    const container = document.getElementById('blockchain');
    container.innerHTML = '';

    blockchainData.forEach((block, index) => {
        const blockCard = document.createElement('div');
        blockCard.className = `block-card ${block.index === 0 ? 'genesis' : ''}`;
        
        const date = new Date(block.timestamp * 1000);
        const formattedDate = date.toLocaleString();
        
        blockCard.innerHTML = `
            <div class="block-header">
                <div class="block-index">Block #${block.index}</div>
                <div class="block-timestamp">${formattedDate}</div>
            </div>
            <div class="block-data">"${block.data}"</div>
            <div class="block-hashes">
                <div class="hash-row">
                    <span class="hash-label">Hash:</span>
                    <span class="hash-value">${block.hash}</span>
                </div>
                <div class="hash-row">
                    <span class="hash-label">Previous:</span>
                    <span class="hash-value">${block.previous_hash}</span>
                </div>
            </div>
            <span class="nonce-badge">Nonce: ${block.nonce}</span>
        `;
        
        container.appendChild(blockCard);
    });
}

async function updateStats() {
    const blockCount = document.getElementById('blockCount');
    const validStatus = document.getElementById('validStatus');
    const difficulty = document.getElementById('difficulty');

    blockCount.textContent = blockchainData.length;

    try {
        const isValid = await invoke('validate_chain');
        validStatus.textContent = isValid ? '✅' : '❌';
        validStatus.parentElement.style.color = isValid ? 'var(--success)' : 'var(--danger)';
    } catch (error) {
        console.error('Failed to validate chain:', error);
    }

    try {
        const diff = await invoke('get_difficulty');
        difficulty.textContent = diff;
    } catch (error) {
        console.error('Failed to get difficulty:', error);
    }
}

document.getElementById('addBlockBtn').addEventListener('click', addBlock);
document.getElementById('refreshBtn').addEventListener('click', loadBlockchain);

document.getElementById('blockData').addEventListener('keypress', (e) => {
    if (e.key === 'Enter') {
        addBlock();
    }
});

loadBlockchain();

