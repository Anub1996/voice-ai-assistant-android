namespace OLX.Api.Models;

public class Category
{
    public int Id { get; set; }
    public string Name { get; set; } = "";
    public string Slug { get; set; } = "";
    public string Icon { get; set; } = "";
    public int? ParentId { get; set; }
    public Category? Parent { get; set; }
    public int SortOrder { get; set; }

    public ICollection<Category> Children { get; set; } = new List<Category>();
    public ICollection<Listing> Listings { get; set; } = new List<Listing>();
}
