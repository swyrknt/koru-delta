# KoruDelta LLM Framework Integrations

This module provides integrations between KoruDelta and popular LLM frameworks.

## Features

### Document Chunking (`chunking.py`)

Intelligent text splitting for RAG pipelines:

```python
from koru_delta.integrations import chunk_document, ChunkingConfig

# Basic usage
chunks = chunk_document(long_text)

# Custom configuration
config = ChunkingConfig(
    chunk_size=1000,
    chunk_overlap=100,
    separators=["\n\n", "\n", ". ", " "],
)
chunks = chunk_document(long_text, config)
```

### Hybrid Search (`hybrid.py`)

Combine vector similarity with causal/temporal filters:

```python
from koru_delta import Database
from koru_delta.integrations import HybridSearcher, CausalFilter

db = Database()
searcher = HybridSearcher(db)

# Hybrid search with time filter
results = await searcher.search(
    query_vector=embedding,
    namespace="documents",
    causal_filter=CausalFilter(
        after_timestamp="2026-01-01T00:00:00Z"
    ),
    vector_weight=0.7,
    causal_weight=0.3,
)

# Time-travel search
results = await searcher.time_travel_search(
    query_vector=embedding,
    timestamp="2026-01-01T00:00:00Z",
)
```

### LangChain Integration (`langchain.py`)

Drop-in VectorStore for LangChain:

```python
from koru_delta import Database
from koru_delta.integrations.langchain import KoruDeltaVectorStore
from langchain_openai import OpenAIEmbeddings

db = Database()
store = KoruDeltaVectorStore(
    db=db,
    namespace="docs",
    embedding_model=OpenAIEmbeddings(),
)

# Use with LangChain
from langchain.chains import RetrievalQA
from langchain_openai import ChatOpenAI

qa = RetrievalQA.from_chain_type(
    llm=ChatOpenAI(),
    retriever=store.as_retriever(),
)
```

### LlamaIndex Integration (`llamaindex.py`)

Native storage backend for LlamaIndex:

```python
from koru_delta import Database
from koru_delta.integrations.llamaindex import KoruDeltaVectorStore
from llama_index.core import VectorStoreIndex

db = Database()
vector_store = KoruDeltaVectorStore(
    db=db,
    namespace="llama_docs"
)

# Create index
index = VectorStoreIndex.from_vector_store(vector_store)
```

## Installation

### Base (chunking + hybrid search)
```bash
pip install koru-delta
```

### With LangChain
```bash
pip install koru-delta[langchain]
```

### With LlamaIndex
```bash
pip install koru-delta[llamaindex]
```

### All frameworks
```bash
pip install koru-delta[frameworks]
```

### Full RAG setup
```bash
pip install koru-delta[rag,frameworks]
```

## RAG Pipeline Example

See `examples/rag_pipeline.py` for a complete RAG implementation:

```python
from koru_delta import Database
from koru_delta.integrations import chunk_document, HybridSearcher

async with Database() as db:
    # Create pipeline
    pipeline = RAGPipeline(db)
    
    # Ingest documents
    await pipeline.ingest_document("document.txt")
    
    # Query
    result = await pipeline.query("What is the main topic?")
    print(result["answer"])
    
    # Time-travel query
    result = await pipeline.query_at(
        "What was the status?",
        "2026-01-01T00:00:00Z"
    )
```
