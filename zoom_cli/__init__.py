"""
zoom-cli: Agent-friendly CLI for the Zoom API.
"""

try:
    from importlib.metadata import version
    __version__ = version("zoom-cli")
except ImportError:
    from importlib_metadata import version
    __version__ = version("zoom-cli")
