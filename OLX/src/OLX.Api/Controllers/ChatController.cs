using System.Security.Claims;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;
using OLX.Api.Data;
using OLX.Api.Models;
using OLX.Api.Services;

namespace OLX.Api.Controllers;

[ApiController, Route("api/[controller]"), Authorize]
public class ChatController : ControllerBase
{
    private readonly AppDbContext _db;
    private readonly NotificationService _notif;

    public ChatController(AppDbContext db, NotificationService notif) { _db = db; _notif = notif; }

    private int Uid => int.Parse(User.FindFirstValue(ClaimTypes.NameIdentifier)!);

    [HttpGet("conversations")]
    public async Task<IActionResult> GetConversations()
    {
        var uid = Uid;
        var convos = await _db.Conversations
            .Include(c => c.Listing).ThenInclude(l => l.Images)
            .Include(c => c.Buyer).Include(c => c.SellerUser)
            .Include(c => c.Messages)
            .Where(c => c.BuyerId == uid || c.SellerId == uid)
            .OrderByDescending(c => c.LastMessageAt).ToListAsync();

        return Ok(convos.Select(c =>
        {
            var other = c.BuyerId == uid ? c.SellerUser : c.Buyer;
            var last = c.Messages.OrderByDescending(m => m.SentAt).FirstOrDefault();
            var unread = c.Messages.Count(m => m.SenderId != uid && !m.IsRead);
            return new ConversationDto(c.Id, c.ListingId, c.Listing.Title,
                c.Listing.Images.OrderBy(i => i.SortOrder).FirstOrDefault()?.Url, c.Listing.Price,
                other.Id, other.FullName, other.ProfilePhotoUrl, last?.Text, c.LastMessageAt, unread);
        }));
    }

    [HttpGet("conversations/{id}/messages")]
    public async Task<IActionResult> GetMessages(long id)
    {
        var uid = Uid;
        var convo = await _db.Conversations.FirstOrDefaultAsync(c => c.Id == id && (c.BuyerId == uid || c.SellerId == uid));
        if (convo == null) return NotFound();

        var msgs = await _db.Messages.Include(m => m.Sender).Where(m => m.ConversationId == id)
            .OrderBy(m => m.SentAt).ToListAsync();

        var unread = msgs.Where(m => m.SenderId != uid && !m.IsRead).ToList();
        unread.ForEach(m => m.IsRead = true);
        if (unread.Any()) await _db.SaveChangesAsync();

        return Ok(msgs.Select(m => new MessageDto(m.Id, m.SenderId, m.Sender.FullName, m.Text, m.ImageUrl, m.IsRead, m.SentAt)));
    }

    [HttpPost("conversations")]
    public async Task<IActionResult> StartConversation(StartConversationRequest req)
    {
        var uid = Uid;
        var listing = await _db.Listings.FindAsync(req.ListingId);
        if (listing == null) return NotFound();
        if (listing.SellerId == uid) return BadRequest(new MessageResponse("Cannot message yourself"));

        var existing = await _db.Conversations.FirstOrDefaultAsync(c => c.ListingId == req.ListingId && c.BuyerId == uid);
        if (existing != null) return Conflict(new IdResponse("Conversation already exists", existing.Id));

        var convo = new Conversation { ListingId = req.ListingId, BuyerId = uid, SellerId = listing.SellerId, LastMessageAt = DateTime.UtcNow };
        _db.Conversations.Add(convo);
        await _db.SaveChangesAsync();

        _db.Messages.Add(new Message { ConversationId = convo.Id, SenderId = uid, Text = req.Message });
        await _db.SaveChangesAsync();

        await _notif.CreateAsync(listing.SellerId, "message", "New message",
            $"New inquiry about {listing.Title}", listing.Id, convo.Id);

        return CreatedAtAction(nameof(GetMessages), new { id = convo.Id }, new IdResponse("Conversation started", convo.Id));
    }

    [HttpPost("conversations/{id}/messages")]
    public async Task<IActionResult> SendMessage(long id, SendMessageRequest req)
    {
        var uid = Uid;
        var convo = await _db.Conversations.Include(c => c.Listing)
            .FirstOrDefaultAsync(c => c.Id == id && (c.BuyerId == uid || c.SellerId == uid));
        if (convo == null) return NotFound();

        var msg = new Message { ConversationId = id, SenderId = uid, Text = req.Text, ImageUrl = req.ImageUrl };
        _db.Messages.Add(msg);
        convo.LastMessageAt = DateTime.UtcNow;
        await _db.SaveChangesAsync();

        var recipientId = convo.BuyerId == uid ? convo.SellerId : convo.BuyerId;
        await _notif.CreateAsync(recipientId, "message", "New message",
            req.Text.Length > 50 ? req.Text[..50] + "..." : req.Text, convo.ListingId, convo.Id);

        return Ok(new MessageDto(msg.Id, msg.SenderId, "", msg.Text, msg.ImageUrl, false, msg.SentAt));
    }
}
