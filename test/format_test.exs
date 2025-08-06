defmodule ExTypst.FormatTest do
  use ExUnit.Case
  import ExTypst.Format
  doctest ExTypst.Format

  describe "table_content/1" do
    test "render integers and strings as expected" do
      users = [
        ["John", 200, 10],
        ["Mary", 500, 100]
      ]

      expected = ~s/"John", "200", "10",\n  "Mary", "500", "100"/

      assert ExTypst.Format.table_content(users) == expected
    end

    test "convert backslashes to line breaks in content blocks" do
      data = [
        ["John", "Software\\Engineer", "USA"],
        ["Mary", "Product\\Manager", "Canada"]
      ]

      expected =
        ~s/"John", [Software \\\nEngineer], "USA",\n  "Mary", [Product \\\nManager], "Canada"/

      assert ExTypst.Format.table_content(data) == expected
    end

    test "table_content_with_breaks/2 converts custom characters to line breaks" do
      data = [
        ["John", "Software|Engineer", "USA"],
        ["Mary", "Product|Manager", "Canada"]
      ]

      expected =
        ~s/"John", [Software \\\nEngineer], "USA",\n  "Mary", [Product \\\nManager], "Canada"/

      assert ExTypst.Format.table_content_with_breaks(data, "|") == expected
    end

    test "table_content_with_breaks/1 uses pipe as default break character" do
      data = [["Alice", "Frontend|Developer", "UK"]]
      expected = ~s/"Alice", [Frontend \\\nDeveloper], "UK"/

      assert ExTypst.Format.table_content_with_breaks(data) == expected
    end

    test "~t sigil allows single backslashes without escaping" do
      data = [["John", ~t"Software\Engineer", "USA"]]
      expected = ~s/"John", [Software \\\nEngineer], "USA"/

      assert ExTypst.Format.table_content(data) == expected
    end
  end
end
