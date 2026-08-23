import pytest
import pi_agent


def test_sum_as_string():
    assert pi_agent.sum_as_string(1, 1) == "2"
