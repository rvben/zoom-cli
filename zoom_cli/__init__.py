"""
zoom-cli: Agent-friendly CLI for the Zoom API.
"""

try:
    from importlib.metadata import version
    __version__ = version("zoomcli")
except ImportError:
    from importlib_metadata import version
    __version__ = version("zoomcli")
