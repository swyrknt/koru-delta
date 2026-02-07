"""Basic tests for koru_delta Python bindings."""

import pytest
import asyncio
from koru_delta import Database, KeyNotFoundError


@pytest.mark.asyncio
async def test_database_create():
    """Test database creation."""
    async with Database() as db:
        assert db is not None


@pytest.mark.asyncio
async def test_put_get():
    """Test basic put/get operations."""
    async with Database() as db:
        await db.put("test", "key1", {"value": 42})
        result = await db.get("test", "key1")
        assert result["value"] == 42


@pytest.mark.asyncio
async def test_key_not_found():
    """Test that missing keys raise KeyNotFoundError."""
    async with Database() as db:
        with pytest.raises(KeyNotFoundError):
            await db.get("test", "nonexistent")


@pytest.mark.asyncio
async def test_contains():
    """Test contains operation."""
    async with Database() as db:
        await db.put("test", "key1", "value")
        assert await db.contains("test", "key1") is True
        assert await db.contains("test", "nonexistent") is False


@pytest.mark.asyncio
async def test_list_keys():
    """Test listing keys in a namespace."""
    async with Database() as db:
        await db.put("test", "key1", "a")
        await db.put("test", "key2", "b")
        keys = await db.list_keys("test")
        assert "key1" in keys
        assert "key2" in keys


@pytest.mark.asyncio
async def test_delete():
    """Test delete operation."""
    async with Database() as db:
        await db.put("test", "key1", "value")
        await db.delete("test", "key1")
        assert await db.contains("test", "key1") is True  # Tombstone exists


@pytest.mark.asyncio
async def test_stats():
    """Test database stats."""
    async with Database() as db:
        await db.put("test", "key1", "value")
        stats = await db.stats()
        assert "key_count" in stats
        assert "namespace_count" in stats
