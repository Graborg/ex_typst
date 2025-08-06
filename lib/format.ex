defmodule ExTypst.Format do
  @moduledoc """
  Contains helper functions for converting elixir datatypes into 
  the format that Typst expects
  """

  @doc """
  Sigil for raw Typst strings that preserves single backslashes.
  Use ~t"..." to write Typst syntax without escaping backslashes.

  ## Examples

      iex> import ExTypst.Format
      iex> ~t"Software\Engineer"
      "Software\\Engineer"
  """
  def sigil_t(string, []), do: string

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

  @spec table_content_with_breaks(list(list(column_data)), String.t()) :: String.t()
  @doc """
  Same as table_content/1 but allows using a custom character to represent line breaks.
  This is useful to avoid having to escape backslashes in Elixir string literals.

  ## Examples

      iex> columns = [["John", "Software|Engineer", "USA"]]
      iex> ExTypst.Format.table_content_with_breaks(columns, "|")
      ~s/"John", [Software \\\\\\nEngineer], "USA"/

      iex> columns = [["Jane", "Product~Manager", "Canada"]]  
      iex> ExTypst.Format.table_content_with_breaks(columns, "~")
      ~s/"Jane", [Product \\\\\\nManager], "Canada"/
  """
  def table_content_with_breaks(columns, break_char \\ "|")
      when is_list(columns) and is_binary(break_char) do
    Enum.map_join(columns, ",\n  ", fn row ->
      Enum.map_join(row, ", ", &format_column_element(&1, break_char))
    end)
  end

  defp format_column_element(e) when is_integer(e), do: add_quotes(e)

  defp format_column_element(e) when is_binary(e),
    do: e |> convert_backslashes_to_linebreaks() |> format_as_content()

  defp format_column_element(unknown), do: unknown |> inspect() |> add_quotes()

  defp format_column_element(e, _break_char) when is_integer(e), do: add_quotes(e)

  defp format_column_element(e, break_char) when is_binary(e),
    do: e |> convert_custom_breaks_to_linebreaks(break_char) |> format_as_content()

  defp format_column_element(unknown, _break_char), do: unknown |> inspect() |> add_quotes()

  defp convert_backslashes_to_linebreaks(s) when is_binary(s) do
    String.replace(s, "\\", " \\\n")
  end

  defp convert_backslashes_to_linebreaks(s), do: to_string(s)

  defp convert_custom_breaks_to_linebreaks(s, break_char)
       when is_binary(s) and is_binary(break_char) do
    String.replace(s, break_char, " \\\n")
  end

  defp convert_custom_breaks_to_linebreaks(s, _break_char), do: to_string(s)

  defp format_as_content(s) do
    if String.contains?(s, " \\\n") do
      "[#{s}]"
    else
      add_quotes(s)
    end
  end

  defp add_quotes(s), do: "\"#{s}\""
end
