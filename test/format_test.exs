defmodule ExTypst.FormatTest do
  use ExUnit.Case
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

    test "convert forward slashes to line breaks in strings" do
      data = [
        ["John", "Software/Engineer", "USA"],
        ["Mary", "Product/Manager", "Canada"]
      ]

      expected = ~s/"John", "Software\\Engineer", "USA",\n  "Mary", "Product\\Manager", "Canada"/

      assert ExTypst.Format.table_content(data) == expected
    end
  end
end
