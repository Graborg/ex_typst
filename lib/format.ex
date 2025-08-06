defmodule ExTypst.Format do
  @moduledoc """
  Contains helper functions for converting elixir datatypes into 
  the format that Typst expects
  """

  @type column_data :: String.t() | integer

  @spec table_content(list(list(column_data))) :: String.t()
  @doc """
  Converts a series of columns mapped as a nested list to a format that can be 
  plugged in an existing table.

  ## Examples

      iex> columns = [["John", 10, 20], ["Alice", 20, 30]]
      iex> ExTypst.Format.table_content(columns)
      ~s/"John", "10", "20",\\n  "Alice", "20", "30"/
  """
  def table_content(columns) when is_list(columns) do
    Enum.map_join(columns, ",\n  ", fn row ->
      Enum.map_join(row, ", ", &format_column_element/1)
    end)
  end

  defp format_column_element(e) when is_integer(e), do: add_quotes(e)
  defp format_column_element(e) when is_binary(e), do: e |> convert_backslashes_to_linebreaks() |> format_as_content()
  defp format_column_element(unknown), do: unknown |> inspect() |> add_quotes()

  defp convert_backslashes_to_linebreaks(s) when is_binary(s) do
    String.replace(s, "\\", " \\ ")
  end
  defp convert_backslashes_to_linebreaks(s), do: to_string(s)

  defp format_as_content(s) do
    if String.contains?(s, " \\ ") do
      "[#{s}]"
    else
      add_quotes(s)
    end
  end

  defp add_quotes(s), do: "\"#{s}\""
end
